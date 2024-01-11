# Chusst, a very simple chess engine in Rust

Simple project to learn about chess programming, Rust and Tauri.

# How to build

Run `cargo tauri dev` from `src-tauri` to build and run the UI, which will allow you to play against the engine.

# Features

* Generates and verifies all the legal moves, including en-passant, castling and promotion.
* Evalutation using alpha-beta pruning.
* Bitboard attack tables.
* UI using Tauri and React + Typescript.
* pgn2yaml: converts [PGN](https://en.wikipedia.org/wiki/Portable_Game_Notation) games to YAML.

# In progress

* chusst-uci: a [UCI](https://en.wikipedia.org/wiki/Universal_Chess_Interface) server.

# To do

* [Fifty-move rule](https://en.wikipedia.org/wiki/Fifty-move_rule).
* pgn2yaml ignores variations.

# License and copyright

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License.

The icon and the chess piece set used in this software have been created by Colin M. L. Burnett.

See [COPYING](COPYING.md).
