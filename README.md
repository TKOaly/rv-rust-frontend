```
 ______     __
|  _ \ \   / /
| |_) \ \ / /
|  _ < \ V /
|_| \_\ \_/
```

> **RV Machine — Terminal Frontend**  
> A blazing-fast terminal interface for the RV machine, built with Rust 🦀

---

## 📦 Deploying

Build the Docker image and run it:

```bash
docker build -t rv-terminal .
docker run rv-terminal
```

---

## 🛠 Development

### Setup

**1.** Start the [`rv-backend`](https://github.com/TKOaly/rv-backend)

**2.** Set the development environment variable:

```bash
export DEVELOPMENT=true
```

**3.** Run the project:

```bash
cargo run
```

> 💡 To exit the program, type `quit` on the login screen.

---

## 🧪 Automated Testing

Start the project in development mode, then launch the test suite:

```bash
# Terminal 1 — start rv-terminal in dev mode
# Terminal 2 — run test suite
```

---

## 🗂 Project Structure

```
rv-tui/
├── .github/
├── ascii/                  # ASCII art assets
├── src/
│   ├── loops/
│   │   ├── management.rs   # Admin loop
│   │   ├── mod.rs          # Main loop
│   │   ├── setting.rs      # Settings loop
│   │   └── user.rs         # User loop
│   ├── input.rs            # Input handling
│   ├── lib.rs
│   ├── main.rs            
│   ├── rv_api.rs           # RV backend API client
│   └── utils.rs            # Shared utilities
├── tests/                  # Basic test to check if rvterminal starts
│   ├── common/
│   │   └── mod.rs
│   └── qaq.rs
├── Dockerfile
├── Cargo.toml
└── README.md
```

---

<p align="center">Built with 💛🖤 and Rust 🦀</p>
