# IOTA Sponsoring Service

A GraphQL-based service for managing IOTA token sponsoring functionality.

## Features

- GraphQL API with GraphiQL IDE
- Dynamic token balance queries with real-time updates
- WebSocket subscriptions for live data streaming
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

## Token Balance Behavior

The service provides real-time token balance updates:

- **Initial Value**: Starts at 50,000 tokens
- **Automatic Updates**: Updates at a random interval between 2 and 5 seconds
- **Decrease Pattern**: Each update decreases the balance by a random amount (100-2000 tokens)
- **Reset Logic**: When the balance would drop below 0, it automatically resets to 50,000
- **Real-time Delivery**: Updates are pushed to subscribers via WebSocket

## Usage

### GraphiQL IDE (Recommended)

Visit http://localhost:8000 to access the interactive GraphQL IDE. This is the easiest way to test both queries and subscriptions.

### GraphQL Queries

**HTTP Endpoint:** POST to http://localhost:8000/graphql

**Get current token balance:**

```graphql
{
  tokenBalance
}
```

**Response:**

```json
{
  "data": {
    "tokenBalance": 48750
  }
}
```

### GraphQL Subscriptions

**WebSocket Endpoint:** ws://localhost:8000/graphql/ws

**Subscribe to real-time balance updates:**

```graphql
subscription {
  tokenBalanceUpdates {
    balance
    timestamp
    action
  }
}
```

**Real-time response stream:**

```json
{
  "data": {
    "tokenBalanceUpdates": {
      "balance": 48750,
      "timestamp": "2025-10-22T10:30:15Z",
      "action": "decreased"
    }
  }
}
```

### Testing with GraphiQL

1. **Open GraphiQL:** http://localhost:8000
2. **For queries** (one-time requests):
   ```graphql
   {
     tokenBalance
   }
   ```
3. **For subscriptions** (real-time updates):
   ```graphql
   subscription {
     tokenBalanceUpdates {
       balance
       timestamp
       action
     }
   }
   ```

### cURL Examples

**Query via HTTP:**

```bash
curl -X POST http://localhost:8000/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ tokenBalance }"}'
```

**Note:** Subscriptions require WebSocket connections and are best tested using GraphiQL.

#### Command-line WebSocket Testing

You can also test subscriptions from the command line using [wscat](https://github.com/websockets/wscat):

1. Install wscat (if you don't have it):
   ```bash
   npm install -g wscat
   ```
2. Connect to the WebSocket endpoint:
   ```bash
   wscat -c ws://localhost:8000/graphql/ws -s graphql-ws
   ```
3. Send the connection init message:
   ```json
   { "type": "connection_init" }
   ```
4. Send the subscription request:
   ```json
   {
     "id": "1",
     "type": "start",
     "payload": {
       "query": "subscription { tokenBalanceUpdates { balance timestamp action } }"
     }
   }
   ```
5. You will receive real-time updates in your terminal as the balance changes.

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
