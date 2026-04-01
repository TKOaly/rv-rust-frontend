```
 ______     __
|  _ \ \   / /
| |_) \ \ / /
|  _ < \ V /
|_| \_\ \_/
```

# Ruokavälitys- frontend  

A blazing-fast terminal interface for TKO-äly Ruokavälitys (Snack kiosk), built with Rust 

---

## Table Of Contents

-   [Deploying](#Deploying)
-   [Development](#Development)
-   [Automated Testing](#Automated-testing)
-   [Project structure](#Project structure)


## Deploying

Build the Docker image and run it:

```bash
docker build -t rv-terminal .
docker run rv-terminal
```

---

## Development

### Running
**1.** Start the [`rv-backend`](https://github.com/TKOaly/rv-backend)

**2.** Set the development environment variable:

```bash
export DEVELOPMENT=true
```

**3.** Run the project:

```bash
cargo run
```

To exit the program, type `quit` on the login screen.

---

## Automated testing

See running in rv-rust-frontend-test (private) 


## Project structure

```
rv-tui/
├── .github/
├── ascii/
├── src/
│   ├── loops/
│   │   ├── management.rs
│   │   ├── mod.rs
│   │   ├── setting.rs
│   │   └── user.rs
│   ├── input.rs
│   ├── lib.rs
│   ├── main.rs            
│   ├── rv_api.rs
│   └── utils.rs
├── tests/                  # Basic test to check if rvterminal starts
│   ├── common/
│   │   └── mod.rs
│   └── qaq.rs
├── Dockerfile
├── Cargo.toml
└── README.md
```
