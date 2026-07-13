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

## ChessGame demo — install and run

The blog series (see `static/blog/`) builds a small ChessGame app made of three
services:

- **chessgame** — the front end (Node + Vite).
- **fisher-server** — the back end (Rust).
- **Stockfish** — the chess engine, run as a Docker container.

### Install the tools

All commands below run inside WSL (Ubuntu) on Windows.

**1. WSL**

Open PowerShell as Administrator and run:

```
wsl --install
```

Reboot when asked, then open the Ubuntu app to finish setup.

**2. Rust**

Inside WSL:

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
cargo --version
```

**3. Node.js**

Install Node via nvm, then check it:

```
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
# reopen the terminal, then:
nvm install --lts
node --version
npm --version
```

**4. Vite**

Vite ships as a dev dependency of the `chessgame` project, so `npm install`
pulls it in. To also have the CLI available globally:

```
npm install -g vite
```

**5. Docker**

Install Docker Desktop for Windows and enable WSL integration in
Settings → Resources → WSL integration. Then, inside WSL:

```
docker --version
```

**6. The Stockfish container**

Pull the engine image once:

```
docker pull ghcr.io/samuraitruong/stockfish-docker:14.1
```

### Run the services

Start each service in its own terminal.

```
# 1. Stockfish engine (Docker) — listens on port 4000
docker run \
  --name stockfish \
  -p 4000:3000 \
  ghcr.io/samuraitruong/stockfish-docker:14.1 \
  stockfish

# 2. Back end (Rust) — from the fisher-server project
cargo run

# 3. Front end (Node + Vite) — from the chessgame project
npm run dev
```

Then open the URL Vite prints (usually `http://localhost:5173`) to play.
