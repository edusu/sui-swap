# SUI SWAP

Web application for token price tracking using a Rust backend on the SUI blockchain.

## Requirements

You need to have either Docker and docker-compose installed or Rust and Cargo.

### Docker Usage

Build the image in the root directory of the project:

```bash
docker build -t sui-swap:0.1.0 .
docker-compose up
```

This will start the server and the three clients.

Manual Usage
You can also run it manually using cargo run. For the server, use:

```bash
cargo run -- -s 
```

And for the clients:

```bash
cargo run -- -c ws://127.0.0.1:8080 SUI
```

```bash
cargo run -- -c ws://127.0.0.1:8080 FUD
```

```bash
cargo run -- -c ws://127.0.0.1:8080 AAA
```

Make sure to start the server before the clients.

These are the three tokens whose information is stored in tokens.json. To add more tokens, simply add more lines to the file. The key can be any identifier, and the value should be the address of the contract on the SUI blockchain.

---

Aplicación web para seguimiento de precio de tokens mediante backend Rust en la blockchain SUI.

## Requisitos

Es necesario tener instalados o bien Docker y docker-compose o bien Rust y Cargo.

### Uso docker

Hacer build de imagen en el directorio raíz del proyecto:

```bash
docker build -t sui-swap:0.1.0 .
```

Ejecutar docker-compose:

```bash
docker-compose up
```

Se levanta el servidor y los tres clientes.

### Uso manual

Se puede también ejecutar mediante cargo run, en mi caso para el servidor uso:

```bash
cargo run -- -s 
```

Y para los clientes:

```bash
cargo run -- -c ws://127.0.0.1:8080 SUI
```

```bash
cargo run -- -c ws://127.0.0.1:8080 FUD
```

```bash
cargo run -- -c ws://127.0.0.1:8080 AAA
```

Importante levantar el servidor antes que los clientes.

Ya que son los tres tokens cuya información he guardado en *tokens.json*. Para añadir más tokens, simplemente añadir más líneas en el archivo, la key puede ser cualquiera, es identificativo, el valor es la dirección del contrato en la blockchain SUI.
