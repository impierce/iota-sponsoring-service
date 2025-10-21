# IOTA Sponsoring Service

A GraphQL-based service for managing IOTA token sponsoring functionality.

## Features

- GraphQL API with GraphiQL IDE
- Token balance queries (mocked)
- Environment-based configuration
- Docker support

## Prerequisites

- Rust 1.70+ (for local development)
- Docker (for containerized deployment)

## Local Development

### 1. Clone and Setup

```bash
git clone <repository-url>
cd iota-sponsoring-service
```

### 2. Environment Configuration

Create a `.env` file in the project root:

```env
SERVER_ADDRESS=127.0.0.1:8000
```

Or set environment variables directly:

```bash
export SERVER_ADDRESS=127.0.0.1:8000
```

### 3. Run Locally

```bash
cargo run
```

The service will start and display:

```
GraphiQL IDE: http://127.0.0.1:8000
Starting server on: 127.0.0.1:8000
```

## Docker Deployment

### 1. Build Docker Image

```bash
docker build -t iota-sponsoring-service .
```

### 2. Run Container

**Basic run:**

```bash
docker run -p 8000:8000 iota-sponsoring-service
```

**With custom configuration:**

```bash
docker run -p 8000:8000 -e SERVER_ADDRESS=0.0.0.0:8000 iota-sponsoring-service
```

### 3. Docker Compose (Optional)

Create `docker-compose.yml`:

```yaml
version: "3.8"
services:
  iota-service:
    build: .
    ports:
      - "8000:8000"
    environment:
      - SERVER_ADDRESS=0.0.0.0:8000
    restart: unless-stopped
```

Run with:

```bash
docker-compose up -d
```

## Usage

### GraphiQL IDE

Visit http://localhost:8000 to access the interactive GraphQL IDE.

### GraphQL Endpoint

Send POST requests to http://localhost:8000/graphql

### Example Queries

**Get token balance:**

```graphql
{
  tokenBalance
}
```

**Response:**

```json
{
  "data": {
    "tokenBalance": 12345
  }
}
```

### cURL Example

```bash
curl -X POST http://localhost:8000/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ tokenBalance }"}'
```

## Configuration

| Environment Variable | Default          | Description                  |
| -------------------- | ---------------- | ---------------------------- |
| `SERVER_ADDRESS`     | `127.0.0.1:8000` | Server bind address and port |

## Development

### Project Structure

```
├── src/
│   └── main.rs                 # Main application entry point
├── presentation/
│   └── axum-graphql/          # GraphQL presentation layer
├── Dockerfile                 # Docker configuration
├── docker-compose.yml         # Docker Compose setup
└── README.md                  # This file
```
