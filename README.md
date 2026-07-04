# A ChessGame with AIAD

A web app where a human plays chess against a computer. The opponent is
[Stockfish](https://stockfishchess.org/), one of the strongest chess engines
available today.

## What we're building

From a software point of view, the goal is simple to state:

> A board shown in the browser. The user moves a piece with the mouse. After
> each move the app sends the position to Stockfish, waits for its reply, and
> shows the engine's move. Repeat until the game ends.

At first glance that's all there is to it. But chess is full of rules and edge
cases — castling, en passant, promotion, check, stalemate, draws — and that is
exactly what makes the project interesting.

## Features

- Play a full game of chess in the browser against Stockfish.
- Move pieces with the mouse: select a piece, then drag-and-drop or click to
  move.
- Start a new game in **standard** mode (the usual opening position) or
  **random** mode (a chosen number of pieces placed at random).
- The board always reflects the current position; refresh it on demand.
- Side panels show the move list, captured pieces, and whose turn it is.
- All chess rules are enforced — castling, en passant, promotion, check,
  checkmate, stalemate, and draws.
