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

Or use Docker Compose:

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

**GraphQL Endpoint:** http://localhost:8000/graphql

### GraphQL Queries

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
   wscat -c http://localhost:8000/graphql -s graphql-ws
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
.
├── application
│   ├── Cargo.toml
│   └── src
│       ├── lib.rs
│       ├── queries
│       │   ├── custom_query.rs
│       │   ├── generic_query_with_sender.rs
│       │   ├── list_all_query.rs
│       │   └── mod.rs
│       ├── services
│       │   ├── balance_management_service.rs
│       │   └── mod.rs
│       └── views
│           ├── client_list.rs
│           ├── client.rs
│           ├── group_list.rs
│           ├── group.rs
│           └── mod.rs
├── Cargo.lock
├── Cargo.toml
├── compose.yaml
├── Dockerfile
├── domain
│   └── balance-management
│       ├── Cargo.toml
│       └── src
│           ├── client
│           │   ├── aggregate.rs
│           │   ├── command.rs
│           │   ├── error.rs
│           │   ├── event.rs
│           │   └── mod.rs
│           ├── group
│           │   ├── aggregate.rs
│           │   ├── command.rs
│           │   ├── error.rs
│           │   ├── event.rs
│           │   └── mod.rs
│           └── lib.rs
├── infrastructure
│   └── composition-root
│       ├── Cargo.toml
│       └── src
│           └── lib.rs
├── LICENSE
├── presentation
│   └── axum-graphql
│       ├── Cargo.toml
│       └── src
│           ├── graphql_endpoint_service.rs
│           ├── lib.rs
│           └── operations
│               ├── mod.rs
│               ├── mutations.rs
│               ├── queries.rs
│               └── subscriptions.rs
├── README.md
└── src
    └── main.rs
```
