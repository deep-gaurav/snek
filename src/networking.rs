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
use futures::task::Spawn;
use serde::{Deserialize, Serialize};
use xwebtransport::current::Connection;
use xwebtransport_core::{datagram::Receive, AcceptUniStream, Connecting, EndpointConnect};

use crate::{
    food::{spawn_food, Food},
    snek::KillSnake,
    CellTag, Direction, GameConfig, GameStates, Host, LastMoveId, Move, MoveId, Moves, Snake,
    SnakeCell, SnakeTag, SpawnDetail,
};

pub enum SendMessage {
    TransportMessage(TransportMessage),
}

type PointInTime = f32;

#[derive(Serialize, Deserialize)]
pub enum TransportMessage {
    Noop,
    // InformPlayers(Vec<PlayerProp>),
    SnakeUpdate(PointInTime, SnakeDetails),
    AddMove(PointInTime, Move),
    StartGame(PointInTime),
    SpawnFood(u32, Vec2),
    KillSnake,
    DespawnFood(u32),
    Ping(f32),
    Pong(f32),
}

#[derive(Serialize, Deserialize)]
pub struct SnakeDetails {
    transform: Transform,
    moves: Moves,
    // spawners: Spawner,
    cells: Vec<SnakeCellDetails>,
}

#[derive(Serialize, Deserialize)]
pub struct SnakeCellDetails {
    cell_tag: CellTag,
    transform: Transform,
    move_id: MoveId,
    direction: crate::Direction,
}

#[derive(Debug, serde::Deserialize)]
pub enum RelayMessage {
    RoomJoined(u32, Vec<u32>),
    UserConnected(u32, Vec<u32>),
    UserDisconnected(u32, Vec<u32>),
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

#[derive(Resource)]
pub struct PingTimer {
    pub timer: Timer,
}

pub struct ConnectionHandler {
    pub self_id: Option<u32>,
    pub room_id: String,
    pub players: Vec<PlayerProp>,
    pub sender: Sender<SendMessage>,
    pub receiver: Receiver<ReceiveMessage>,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerProp {
    pub last_update_time: Option<PointInTime>,
    pub start_time: Option<(PointInTime, PointInTime)>,
    pub user_id: u32,
    pub color: Color,
    pub score: u32,
    pub highest_score: u32,
}

#[derive(Event)]
pub struct PlayersChanged {
    pub players: Vec<PlayerProp>,
    pub self_player: Option<u32>,
}

#[derive(Event)]
pub struct SnakeUpdate {
    update_time: PointInTime,
    user_id: u32,
    snake_details: SnakeDetails,
}

#[derive(Event)]
pub struct AddMove {
    update_time: PointInTime,
    user_id: u32,
    _move: Move,
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
    let room_id_c = room_id.to_string();
    let task = thread_pool
        .spawn(async move {
            let connection = endpoint
                .connect(&format!(
                    "https://web-room-relay.deepwith.in:4433/room/{room_id_c}",
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
        self_id: None,
        players: vec![],
        sender: sender_tx,
        receiver: receiver_rx,
        room_id: room_id.to_string(),
    });
}

pub fn receive_msgs(
    config: Res<GameConfig>,
    mut connection_handler: ResMut<ConnectionState>,
    mut next_state: ResMut<NextState<GameStates>>,
    current_state: Res<State<GameStates>>,
    mut snake_update: EventWriter<SnakeUpdate>,
    mut add_move: EventWriter<AddMove>,
    mut players_changed_ev: EventWriter<PlayersChanged>,
    mut host: Query<Entity, With<Host>>,
    food: Query<(Entity, &Food)>,
    mut commands: Commands,
    time: Res<Time>,
    mut snake_killer: EventWriter<KillSnake>,
    snakes: Query<(Entity, &SnakeTag)>,
) {
    match connection_handler.as_mut() {
        ConnectionState::NotConnected => {}
        ConnectionState::Connected(connection) => {
            for msg in connection.receiver.try_iter() {
                print!("Connection established");
                match msg {
                    ReceiveMessage::ConnectionEstablished => {
                        info!("Connection established");
                        next_state.set(GameStates::Lobby);
                    }
                    ReceiveMessage::DatagramReceived(data) => {
                        let msg = bincode::deserialize::<RelayMessage>(&data);
                        if let Ok(msg) = msg {
                            match msg {
                                RelayMessage::RoomJoined(user_id, users) => {
                                    connection.sender.send(SendMessage::TransportMessage(
                                        TransportMessage::Noop,
                                    ));
                                    info!("Joined room with id {}", user_id);
                                    connection.self_id = Some(user_id);
                                    if connection.players.is_empty() {
                                        let color = Color::Hsla {
                                            hue: seeded_random::Random::from_seed(
                                                seeded_random::Seed::unsafe_new(user_id as u64),
                                            )
                                            .gen::<f32>()
                                                * 360.,
                                            saturation: 1.,
                                            lightness: 0.5,
                                            alpha: 1.,
                                        };
                                        connection.players.push(PlayerProp {
                                            last_update_time: None,
                                            user_id,
                                            color,
                                            start_time: None,
                                            score:0,
                                            highest_score:0,
                                        });
                                    }
                                    for user in users.iter() {
                                        let color = Color::Hsla {
                                            hue: seeded_random::Random::from_seed(
                                                seeded_random::Seed::unsafe_new(*user as u64),
                                            )
                                            .gen::<f32>()
                                                * 360.,
                                            saturation: 1.,
                                            lightness: 0.5,
                                            alpha: 1.,
                                        };
                                        connection.players.push(PlayerProp {
                                            user_id: *user,
                                            color,
                                            start_time: None,
                                            last_update_time: None,
                                            score:0,
                                            highest_score:0,
                                        });
                                        players_changed_ev.send(PlayersChanged {
                                            players: connection.players.clone(),
                                            self_player: connection.self_id,
                                        });
                                    }
                                    players_changed_ev.send(PlayersChanged {
                                        players: connection.players.clone(),
                                        self_player: connection.self_id,
                                    });
                                    if !users.is_empty() {
                                        for host in host.iter() {
                                            commands.entity(host).despawn();
                                        }
                                    }
                                }
                                RelayMessage::UserConnected(id, users) => {
                                    info!("User connected {id}");
                                    let color = Color::Hsla {
                                        hue: seeded_random::Random::from_seed(
                                            seeded_random::Seed::unsafe_new(id as u64),
                                        )
                                        .gen::<f32>()
                                            * 360.,
                                        saturation: 1.,
                                        lightness: 0.5,
                                        alpha: 1.,
                                    };
                                    connection.players.push(PlayerProp {
                                        user_id: id,
                                        color,
                                        start_time: None,
                                        last_update_time: None,
                                        score:0,
                                        highest_score:0,
                                    });
                                    players_changed_ev.send(PlayersChanged {
                                        players: connection.players.clone(),
                                        self_player: connection.self_id,
                                    });
                                }
                                RelayMessage::UserDisconnected(id, users) => {
                                    info!("User Disconnected {id}");
                                    let p_index =
                                        connection.players.iter().position(|p| p.user_id == id);

                                    if connection.self_id == users.iter().next().cloned()
                                        && host.is_empty()
                                    {
                                        info!("I'm host now!");
                                        commands.spawn(Host);
                                    }
                                    if let Some(player_index) = p_index {
                                        connection.players.remove(player_index);
                                        players_changed_ev.send(PlayersChanged {
                                            players: connection.players.clone(),
                                            self_player: connection.self_id,
                                        });
                                        info!("Removed player len {}", connection.players.len())
                                    }
                                }
                                RelayMessage::UserMessage(user_id, msg) => {
                                    let transport_msg =
                                        bincode::deserialize::<TransportMessage>(&msg);
                                    if let Ok(transport_msg) = transport_msg {
                                        match transport_msg {
                                            TransportMessage::Noop => {}
                                            TransportMessage::Ping(t) => {
                                                connection.sender.send(
                                                    SendMessage::TransportMessage(
                                                        TransportMessage::Pong(t),
                                                    ),
                                                );
                                            }
                                            TransportMessage::Pong(t) => {
                                                info!("Ping {}", time.elapsed_seconds() - t);
                                            }
                                            TransportMessage::SnakeUpdate(
                                                update_time,
                                                snake_details,
                                            ) => snake_update.send(SnakeUpdate {
                                                update_time: update_time,
                                                user_id: user_id,
                                                snake_details,
                                            }),
                                            TransportMessage::AddMove(update_time, _move) => {
                                                let player = connection
                                                    .players
                                                    .iter_mut()
                                                    .find(|p| p.user_id == user_id);
                                                if let Some(player) = player {
                                                    player.last_update_time = Some(update_time);
                                                };
                                                add_move.send(AddMove {
                                                    user_id,
                                                    _move,
                                                    update_time,
                                                })
                                            }

                                            // TransportMessage::InformPlayers(players) => {
                                            //     connection.players = players;
                                            //     for host in host.iter() {
                                            //         commands.entity(host).despawn();
                                            //     }
                                            // }
                                            TransportMessage::StartGame(start_time) => {
                                                next_state.set(GameStates::GamePlay);
                                            }
                                            TransportMessage::SpawnFood(food_id, food_pos) => {
                                                commands.spawn(spawn_food(
                                                    food_id,
                                                    config.cell_size,
                                                    food_pos.x,
                                                    food_pos.y,
                                                ));
                                            }
                                            TransportMessage::DespawnFood(id) => {
                                                let food = food.iter().find(|f| f.1 .0 == id);
                                                if let Some(food) = food {
                                                    commands.entity(food.0).despawn_recursive();
                                                }
                                            }
                                            TransportMessage::KillSnake => {
                                                if let Some(snek) = snakes.iter().find(|p| {
                                                    p.1 == &SnakeTag::OtherPlayerSnake(user_id)
                                                }) {
                                                    snake_killer
                                                        .send(KillSnake { snake_id: snek.0 });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    ReceiveMessage::ConnectionError => {}
                    ReceiveMessage::ChannelReceiveError => {}
                }
            }
        }
    }
}

pub fn ping_send(
    mut ping_tick: ResMut<PingTimer>,
    time: Res<Time>,
    connection_handler: Res<ConnectionState>,
) {
    ping_tick.timer.tick(time.delta());
    if ping_tick.timer.finished() {
        if let ConnectionState::Connected(connection) = connection_handler.as_ref() {
            connection
                .sender
                .send(SendMessage::TransportMessage(TransportMessage::Ping(
                    time.elapsed_seconds(),
                )));
        }
    }
}

pub fn send_snake_send(
    transforms: Query<(&Transform), Or<(With<SnakeTag>, With<CellTag>)>>,
    moves: Query<(&Moves)>,
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
    let Ok(moves) = moves.get(self_snake) else {
        return;
    };
    let moves = moves.clone();

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
        transform: snake_tranform,
        cells: snake_cells,
        moves,
    };
    match connection_handler.as_ref() {
        ConnectionState::NotConnected => {}
        ConnectionState::Connected(connection) => {
            if let Err(err) = connection.sender.send(SendMessage::TransportMessage(
                TransportMessage::SnakeUpdate(time.elapsed_seconds(), snake_details),
            )) {
                warn!("{err:?}")
            }
        }
    }
}

pub fn update_snake(
    mut snake_update: EventReader<SnakeUpdate>,
    mut commands: Commands,
    mut snake: Query<(Entity, &SnakeTag)>,
    cells: Query<(Entity, &CellTag)>,
    config: Res<GameConfig>,
    mut moves: Query<&mut Moves>,
    mut move_id: Query<&mut MoveId>,
    mut direction: Query<&mut Direction>,
    mut transmform: Query<&mut Transform, Or<(With<CellTag>, With<SnakeTag>)>>,
    mut connection_handler: ResMut<ConnectionState>,
    time: Res<Time>,
) {
    let ConnectionState::Connected(connection) = connection_handler.as_mut() else {
        return;
    };
    let cell_size = config.cell_size;
    let collider_size = (config.cell_size.0 / 2.0, config.cell_size.1 / 2.0);

    for event in snake_update.into_iter() {
        let Some(player) = connection
            .players
            .iter_mut()
            .find(|p| p.user_id == event.user_id)
        else {
            continue;
        };
        if let Some(mut last_up) = player.last_update_time {
            if event.update_time < last_up {
                info!("Skipping late event");
                continue;
            }
            player.last_update_time = Some(event.update_time);
        } else {
            player.last_update_time = Some(event.update_time);
        }
        let snake = snake
            .iter_mut()
            .find(|snake| snake.1 == &SnakeTag::OtherPlayerSnake(event.user_id));
        if let Some(mut snake) = snake {
            *transmform.get_mut(snake.0).unwrap() = event.snake_details.transform;
            *moves.get_mut(snake.0).unwrap() = event.snake_details.moves.clone();
            for cell in event.snake_details.cells.iter() {
                let cell_entity = cells.iter().find(|p| p.1 == &cell.cell_tag);
                let compensation_time =
                    if let Some((start_time_player, start_time_self)) = player.start_time {
                        let extra = (time.elapsed_seconds() - start_time_self)
                            - (event.update_time - start_time_player);
                        // info!("Lagged {}", extra);
                        if extra < 0. {
                            player.start_time = Some((event.update_time, time.elapsed_seconds()));
                        }
                        extra
                    } else {
                        0.
                    }
                    .clamp(0., f32::INFINITY);
                let direction_vec3: Vec3 = cell.direction.clone().into();
                let compensation_transform: Vec3 =
                    compensation_time * config.speed * direction_vec3;
                if let Some(cell_entity) = cell_entity {
                    *transmform.get_mut(cell_entity.0).unwrap() = cell
                        .transform
                        .with_translation(cell.transform.translation + compensation_transform);
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
                                    color: player.color,
                                    custom_size: Some(Vec2::new(cell_size.0, cell_size.1)),
                                    ..default()
                                },
                                transform: cell.transform.clone().with_translation(
                                    cell.transform.translation + compensation_transform,
                                ),
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
                .spawn((Snake {
                    tag: SnakeTag::OtherPlayerSnake(event.user_id),
                    spatial: SpatialBundle::from_transform(event.snake_details.transform),

                    lastmove: LastMoveId(0),
                    moves: event.snake_details.moves.clone(),
                },))
                .with_children(|parent| {
                    for cell in event.snake_details.cells.iter() {
                        parent.spawn(SnakeCell {
                            cell_tag: cell.cell_tag,
                            collider: Collider::cuboid(collider_size.0, collider_size.1),
                            sensor: Sensor,
                            direction: cell.direction.clone(),
                            sprite: SpriteBundle {
                                sprite: Sprite {
                                    color: player.color,
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
            player.start_time = Some((event.update_time, time.elapsed_seconds()));
        }
    }
}

pub fn sync_add_move(
    mut moves: Query<(&mut Moves, &SnakeTag)>,
    mut add_move: EventReader<AddMove>,
) {
    for _move in add_move.iter() {
        for (mut moves, snake) in moves.iter_mut() {
            if let SnakeTag::OtherPlayerSnake(id) = snake {
                if id == &_move.user_id {
                    moves.moves.push(_move._move.clone());
                }
            }
        }
    }
}
