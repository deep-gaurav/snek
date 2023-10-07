// pub fn connect

use std::default;

use bevy::{
    prelude::*,
    sprite::Sprite,
    tasks::{AsyncComputeTaskPool, Task},
    time::{Time, Timer},
};
use bevy_rapier2d::prelude::{Collider, Sensor};
use flume::{Receiver, Sender};
use serde::{Deserialize, Serialize};
use xwebtransport::current::Connection;
use xwebtransport_core::{datagram::Receive, Connecting, EndpointConnect};

use crate::{
    CellTag, Direction, GameConfig, GameStates, LastMoveId, MoveId, Moves, Snake, SnakeCell,
    SnakeTag, Spawner,
};

pub enum SendMessage {
    TransportMessage(TransportMessage),
}

#[derive(Serialize, Deserialize)]
pub enum TransportMessage {
    SnakeUpdate(SnakeDetails),
}

#[derive(Serialize, Deserialize)]
pub struct SnakeDetails {
    elaps: f64,
    transform: Transform,
    moves: Moves,
    spawners: Spawner,
    cells: Vec<SnakeCellDetails>,
}

#[derive(Serialize, Deserialize)]
pub struct SnakeCellDetails {
    cell_tag: CellTag,
    transform: Transform,
    move_id: MoveId,
    direction: Direction,
}

#[derive(Debug, serde::Deserialize)]
pub enum RelayMessage {
    UserConnected(u32),
    UserDisconnected(u32),
    UserMessage(u32, Vec<u8>),
}

#[derive(PartialEq)]
pub enum ReceiveMessage {
    ConnectionEstablished,
    DatagramReceived(Vec<u8>),
    ConnectionError,
    ChannelReceiveError,
}

#[derive(Resource)]
pub enum ConnectionState {
    NotConnected,
    Connected(ConnectionHandler),
}

#[derive(Resource)]
pub struct SnakeSyncTimer {
    pub timer: Timer,
}

pub struct ConnectionHandler {
    sender: Sender<SendMessage>,
    receiver: Receiver<ReceiveMessage>,
}

#[derive(Component)]
pub struct LastUpdatedAt(f64);

#[derive(Event)]
pub struct SnakeUpdate {
    user_id: u32,
    snake_details: SnakeDetails,
}

#[derive(Component)]
pub struct ReceivedMsgTask(Task<ReceiveMessage>);

pub fn connect_transport(room_id: &str, mut connection_handler: ResMut<ConnectionState>) {
    let thread_pool = AsyncComputeTaskPool::get();
    let endpoint = xwebtransport::current::Endpoint {
        ..Default::default()
    };
    let (sender_tx, sender_rx) = flume::unbounded();
    let (receiver_tx, receiver_rx) = flume::unbounded();
    let room_id = room_id.to_string();
    let task = thread_pool
        .spawn(async move {
            let connection = endpoint
                .connect(&format!(
                    "https://web-room-relay.deepwith.in:4433/room/{room_id}"
                ))
                .await;
            if let Ok(connection) = connection {
                if let Ok(connection) = connection.wait_connect().await {
                    if let Err(err) = receiver_tx.send(ReceiveMessage::ConnectionEstablished) {
                        warn!("Failed to send rcv {err:?}")
                    }
                    let mut send_msg_fut = None;
                    loop {
                        let send_msg_fut_local = send_msg_fut.take();
                        let resp = futures::future::select(
                            sender_rx.recv_async(),
                            match send_msg_fut_local {
                                Some(val) => val,
                                None => connection.receive_datagram(),
                            },
                        )
                        .await;
                        match resp {
                            futures::future::Either::Left((send_msg, data_gram_fut)) => {
                                send_msg_fut = Some(data_gram_fut);
                                if let Ok(msg) = send_msg {
                                    if let SendMessage::TransportMessage(msg) = msg {
                                        let bin = bincode::serialize(&msg)
                                            .ok()
                                            // .and_then(|val| zstd::encode_all(val, 0).ok())
                                            ;
                                        if let Some(bin) = bin {
                                            use xwebtransport_core::datagram::Send;
                                            connection.send_datagram(&bin).await;
                                        }
                                    }
                                }
                            }
                            futures::future::Either::Right((datagram, send_msg_fut)) => {
                                let res = match datagram {
                                    Ok(datagram) => receiver_tx
                                        .send(ReceiveMessage::DatagramReceived(datagram.to_vec())),
                                    Err(err) => receiver_tx.send(ReceiveMessage::ConnectionError),
                                };
                                if let Err(err) = res {
                                    warn!("{err:?}")
                                }
                            }
                        }
                    }
                }
            }
        })
        .detach();
    *connection_handler.as_mut() = ConnectionState::Connected(ConnectionHandler {
        sender: sender_tx,
        receiver: receiver_rx,
    });
}

pub fn receive_msgs(
    connection_handler: Res<ConnectionState>,
    mut next_state: ResMut<NextState<GameStates>>,
    current_state: Res<State<GameStates>>,
    mut snake_update: EventWriter<SnakeUpdate>,
) {
    match connection_handler.as_ref() {
        ConnectionState::NotConnected => {}
        ConnectionState::Connected(connection) => {
            for msg in connection.receiver.try_iter() {
                print!("Connection established");
                if current_state.is_entry_menu() && msg == ReceiveMessage::ConnectionEstablished {
                    next_state.set(GameStates::GamePlay);
                }
                if let ReceiveMessage::DatagramReceived(data) = msg {
                    let msg = bincode::deserialize::<RelayMessage>(&data);
                    if let Ok(msg) = msg {
                        if let RelayMessage::UserMessage(user_id, msg) = msg {
                            let transport_msg = bincode::deserialize::<TransportMessage>(&msg);
                            if let Ok(transport_msg) = transport_msg {
                                if let TransportMessage::SnakeUpdate(snake_details) = transport_msg
                                {
                                    snake_update.send(SnakeUpdate {
                                        user_id: user_id,
                                        snake_details,
                                    })
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn send_snake_send(
    transforms: Query<(&Transform), Or<(With<SnakeTag>, With<CellTag>)>>,
    moves_spawners: Query<(&Moves, &Spawner)>,
    moveid_direc: Query<(&Direction, &MoveId, &CellTag)>,
    snake: Query<(Entity, &SnakeTag)>,
    snake_cells: Query<(&Parent, Entity), With<CellTag>>,
    connection_handler: Res<ConnectionState>,
    mut spawner_tick: ResMut<SnakeSyncTimer>,
    time: Res<Time>,
) {
    spawner_tick.timer.tick(time.delta());
    if !spawner_tick.timer.finished() {
        return;
    }
    let Some((self_snake, snake_tag)) =
        snake.iter().find(|val| val.1 == &SnakeTag::SelfPlayerSnake)
    else {
        return;
    };
    let Ok(snake_tranform) = transforms.get(self_snake).cloned() else {
        return;
    };
    let Ok((moves, spawner)) = moves_spawners.get(self_snake) else {
        return;
    };
    let moves = moves.clone();
    let spawners = spawner.clone();

    let snake_cells = snake_cells
        .iter()
        .filter(|cell| cell.0.get() == self_snake)
        .map(|(par, cell)| {
            let transform = transforms.get(cell).unwrap();
            let (dir, move_id, tag) = moveid_direc.get(cell).unwrap();
            SnakeCellDetails {
                cell_tag: tag.clone(),
                transform: transform.clone(),
                move_id: MoveId(move_id.0),
                direction: crate::Direction(dir.0),
            }
        })
        .collect();
    let snake_details = SnakeDetails {
        elaps: time.elapsed_seconds_f64(),
        transform: snake_tranform,
        cells: snake_cells,
        moves,
        spawners,
    };
    match connection_handler.as_ref() {
        ConnectionState::NotConnected => {}
        ConnectionState::Connected(connection) => {
            if let Err(err) = connection.sender.send(SendMessage::TransportMessage(
                TransportMessage::SnakeUpdate(snake_details),
            )) {
                warn!("{err:?}")
            }
        }
    }
}

pub fn update_snake(
    mut snake_update: EventReader<SnakeUpdate>,
    mut commands: Commands,
    snake: Query<(Entity, &SnakeTag, &LastUpdatedAt)>,
    cells: Query<(Entity, &CellTag)>,
    config: Res<GameConfig>,
    mut moves: Query<&mut Moves>,
    mut spawners: Query<&mut Spawner>,
    mut move_id: Query<&mut MoveId>,
    mut direction: Query<&mut Direction>,
    mut transmform: Query<&mut Transform, Or<(With<CellTag>, With<SnakeTag>)>>,
) {
    let cell_size = config.cell_size;
    let collider_size = (config.cell_size.0 / 2.0, config.cell_size.1 / 2.0);

    for event in snake_update.into_iter() {
        let snake = snake
            .iter()
            .find(|snake| snake.1 == &SnakeTag::OtherPlayerSnake(event.user_id));
        if let Some(snake) = snake {
            if (snake.2 .0 > event.snake_details.elaps) {
                return;
            }
            *transmform.get_mut(snake.0).unwrap() = event.snake_details.transform;
            *moves.get_mut(snake.0).unwrap() = event.snake_details.moves.clone();
            *spawners.get_mut(snake.0).unwrap() = event.snake_details.spawners.clone();
            for cell in event.snake_details.cells.iter() {
                let cell_entity = cells.iter().find(|p| p.1 == &cell.cell_tag);
                if let Some(cell_entity) = cell_entity {
                    *transmform.get_mut(cell_entity.0).unwrap() = cell.transform;
                    *direction.get_mut(cell_entity.0).unwrap() = cell.direction.clone();
                    *move_id.get_mut(cell_entity.0).unwrap() = MoveId(cell.move_id.0);
                } else {
                    let tmp_cell = commands
                        .spawn(SnakeCell {
                            cell_tag: cell.cell_tag,
                            collider: Collider::cuboid(collider_size.0, collider_size.1),
                            sensor: Sensor,
                            direction: cell.direction.clone(),
                            sprite: SpriteBundle {
                                sprite: Sprite {
                                    color: Color::rgb(0.25, 0.25, 0.75),
                                    custom_size: Some(Vec2::new(cell_size.0, cell_size.1)),
                                    ..default()
                                },
                                transform: cell.transform.clone(),
                                ..default()
                            },
                            move_id: MoveId(cell.move_id.0),
                        })
                        .id();
                    commands.entity(snake.0).add_child(tmp_cell);
                }
            }
        } else {
            let snake = commands
                .spawn((
                    Snake {
                        tag: SnakeTag::OtherPlayerSnake(event.user_id),
                        spatial: SpatialBundle::from_transform(event.snake_details.transform),

                        lastmove: LastMoveId(0),
                        moves: event.snake_details.moves.clone(),
                        spawners: event.snake_details.spawners.clone(),
                    },
                    LastUpdatedAt(event.snake_details.elaps),
                ))
                .with_children(|parent| {
                    for cell in event.snake_details.cells.iter() {
                        parent.spawn(SnakeCell {
                            cell_tag: cell.cell_tag,
                            collider: Collider::cuboid(collider_size.0, collider_size.1),
                            sensor: Sensor,
                            direction: cell.direction.clone(),
                            sprite: SpriteBundle {
                                sprite: Sprite {
                                    color: Color::rgb(0.25, 0.25, 0.75),
                                    custom_size: Some(Vec2::new(cell_size.0, cell_size.1)),
                                    ..default()
                                },
                                transform: cell.transform.clone(),
                                ..default()
                            },
                            move_id: MoveId(cell.move_id.0),
                        });
                    }
                })
                .id();
        }
    }
}
