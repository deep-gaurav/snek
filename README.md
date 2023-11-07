# Snek with Friends

[Play in Browser](https://snek.deepwith.in)

Simple multiplayer snake game built with Bevy, WebTransport which runs on both web and natively.

![snek](https://github.com/deep-gaurav/snek/assets/28472450/6bb5fd32-8ebc-4781-abed-7bf3b5b47dac)

## Description

It's a simple game built to learn bevy(0.11) and see how well WebTransport performs onweb.
All of logic is in bevy client, There's a webtransport server which acts as relay server which provides room functionality and to broadcast messages received from one user to every other user in same room.

