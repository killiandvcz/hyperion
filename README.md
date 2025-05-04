# Hyperion - An Endpoint-First Database

Hyperion is a next-generation database system that reimagines data storage and retrieval through an innovative "endpoint-first" approach. Unlike traditional databases that focus on documents, records, or nodes as primary units, Hyperion treats individual data endpoints (path-value pairs) as the fundamental storage unit.

## Core Concept: Endpoint-First

In Hyperion, every piece of data is represented as a path-endpoint pair:

```
users.u-123456.username = "alice"
users.u-123456.email = "alice@example.com"
users.u-123456.profile.bio = "Software engineer and hobby photographer"
```

This approach allows for:

- **Maximum flexibility** - Any data structure can be represented naturally
- **Path-native operations** - Direct access to any data point without document overhead
- **Unified addressing** - One consistent way to reference any piece of data
- **Natural evolution** - Schema-free growth without predefined structures
- **Paradigm unification** - Key-value, document, relational, and graph models in one system

## Current State and Performance

Hyperion is currently in an early alpha stage. The core components are functional, but the system is under active development.

Initial benchmarks show impressive performance characteristics:

- **Read operations**: 100,000+ operations per second
- **Write operations (with batching)**: 40,000+ operations per second
- **Path pattern matching**: Up to 3 million operations per second
- **Wildcard queries**: 250,000+ operations per second

## What's Implemented vs. Future Plans

### Already Implemented ✅

- **Core Data Model**: Path, Value, and Entity structures
- **Storage Engine**: In-memory and persistent storage with LSM-Tree based format
- **Basic Path Operations**: CRUD operations on individual endpoints
- **Entity Reconstruction**: Automatic reconstruction of "documents" from individual endpoints
- **Simple Pattern Matching**: Support for single-level (*) and multi-level (**) wildcards
- **Efficient Indexing**: Prefix and wildcard path indexing for efficient queries
- **Performance Optimization**: Batching mechanism for index operations
- **Basic Query Language**: Simple query parser and executor for fundamental operations

### Coming Soon 🚀

- **Full HyperionQL Implementation**: Complete query language according to the RFC
- **Advanced Query Optimization**: Intelligent query planning and execution
- **Improved Concurrency**: Enhanced transaction support
- **Distribution**: Sharding and cluster management
- **Vectorized Paths**: Similarity-based path searches
- **Bio-inspired Optimization**: Self-learning performance tuning

## HyperionQL: The Complete Vision

HyperionQL is designed to be a radically new way of interacting with data, combining simplicity of expression with powerful capabilities. Unlike traditional query languages tied to specific data models (SQL for relational, Cypher for graphs, etc.), HyperionQL provides a unified interface for all data paradigms.

All operations in HyperionQL follow a consistent transaction-based structure:

```
{
  // Operations
  
  return <expression>
}
```

### Basic Operations (Already Implemented ✅)

```
{
  // Store a simple value
  users.u-123456.username = "alice"
  
  // Retrieve a specific endpoint
  return users.u-123456.username
  
  // Simple wildcard query
  return users.*.email
}
```

### Entity Operations (Partially Implemented ⚙️)

Hyperion automatically reconstructs entities from related endpoints:

```
{
  // Retrieve a complete user entity
  return users.u-123456
  
  // Automatic reconstruction finds all endpoints with this prefix
  // and builds a structured representation
}
```

In the future, you'll be able to shape the returned entities:

```
{
  // Shape the reconstruction with specific attributes (Coming Soon 🚀)
  return users.u-123456.{
    username,
    display_name: profile.full_name,
    contact: {
      email,
      phone
    }
  }
}
```

### Advanced Filtering (Coming Soon 🚀)

```
{
  // Find all active users
  return users where their.active == true
  
  // Find users with high login count
  return users where their.login_count > 100
}
```

### Relationships and Graph Queries (Coming Soon 🚀)

HyperionQL will excel at relationship-based queries:

```
{
  // Find friends of friends
  let user = users.u-123456
  let friends_of_friends = []
  
  for (let friend_id of user.friends) {
    let friend = users[friend_id]
    for (let fof_id of friend.friends) {
      if (fof_id != user.id && !user.friends.includes(fof_id)) {
        friends_of_friends.push(fof_id)
      }
    }
  }
  
  return {
    user: user.username,
    friends_of_friends: friends_of_friends.map(id => users[id].username)
  }
}
```

### Batch Operations (Coming Soon 🚀)

```
{
  // Create multiple endpoints at once
  let id = "u-" + uuid()
  
  batch(users[id]) {
    set ["username"] = "new_user"
    set ["email"] = "new@example.com"
    set ["created_at"] = now()
    set ["profile.bio"] = "New user bio"
  }
  
  return id
}
```

### Vector Search (Coming Soon 🚀)

```
{
  // Find users with similar profiles
  return vector_search("users.*.profile", 
                      encode_vector(users.u-123456.profile),
                      limit: 5,
                      min_similarity: 0.8)
}
```

### Time Travel and Versioning (Coming Soon 🚀)

```
{
  // Get previous versions of data
  return history(users.u-123456.profile.bio, 
                from: "2024-01-01", 
                to: "2024-04-01")
  
  // Query a point-in-time state
  return snapshot(users.u-123456, as_of: "2024-03-15T00:00:00Z")
}
```

### Streaming Changes (Coming Soon 🚀)

```
{
  // Subscribe to changes on a specific entity
  subscribe(users.u-123456)
  
  // Subscribe with a filter
  subscribe(users.*.status) where new_value == "online"
}
```

### Path-Based Access Control (Coming Soon 🚀)

```
{
  // Define fine-grained access rules
  define_access_control(users.*.profile, {
    read: "public",
    write: "owner",
    rules: [
      {
        role: "friend",
        condition: "parent().parent().id in auth.user.friends",
        operations: ["read"]
      }
    ]
  })
}
```

## Rich Query Examples

### Social Media Feed (Coming Soon 🚀)

```
{
  // Get a user's feed with posts from friends
  let user = users.u-123456
  
  // Get user's friends' recent posts with likes and comments
  let feed = posts where 
    their.author in user.friends && 
    their.created_at > (now() - days(7))
  
  // Sort by creation time
  feed = feed.sort((a, b) => b.created_at - a.created_at)
  
  // Format each post with additional data
  return feed.map(post => {
    return {
      id: post.id,
      content: post.content,
      author: users[post.author].{
        username,
        avatar
      },
      created_at: post.created_at,
      likes: count(likes) where their.post_id == post.id,
      comments: comments where their.post_id == post.id {
        id,
        content,
        author: users[their.author].username
      }
    }
  })
}
```

### E-commerce Inventory Management (Coming Soon 🚀)

```
{
  // Find products running low on inventory
  let low_inventory = products where 
    their.stock < their.reorder_threshold && 
    !their.discontinued
  
  // Calculate reorder quantities
  let orders = low_inventory.map(product => {
    // Update last checked timestamp
    products[product.id].inventory_last_checked = now()
    
    // Calculate optimal order quantity 
    let annual_demand = products[product.id].sales_data.annual_units || 0
    let order_cost = products[product.id].supplier_data.order_cost || 10
    let holding_cost_percent = products[product.id].inventory_data.holding_cost_percent || 0.2
    let unit_cost = products[product.id].price_data.unit_cost || 1
    
    let holding_cost = unit_cost * holding_cost_percent
    let eoq = Math.sqrt((2 * annual_demand * order_cost) / holding_cost)
    
    // Round up to case pack quantity
    let case_pack = products[product.id].supplier_data.case_pack || 1
    let order_quantity = Math.ceil(eoq / case_pack) * case_pack
    
    return {
      product_id: product.id,
      product_name: product.name,
      current_stock: product.stock,
      recommended_order: order_quantity,
      supplier_id: product.supplier_data.preferred_supplier
    }
  })
  
  return {
    review_date: now(),
    total_products_to_reorder: orders.length,
    orders: orders,
    total_estimated_cost: sum(orders.map(order => 
      order.recommended_order * products[order.product_id].price_data.unit_cost
    ))
  }
}
```

### IoT Sensor Analytics (Coming Soon 🚀)

```
{
  // Find all temperature sensors reporting high values
  let high_temp_sensors = endpoints(devices.*.sensors.*.current_value) 
    where path.includes("temperature") && value > 90
  
  // Get complete information about these devices
  let at_risk_devices = high_temp_sensors.map(sensor_path => {
    // Extract device ID from path
    let device_id = sensor_path.split('.')[1]
    
    // Record the alert in the device history
    devices[device_id].alerts[now().toISOString()] = {
      type: "high_temperature",
      sensor_path: sensor_path,
      value: endpoint(sensor_path)
    }
    
    // Return device information with the sensor data
    return {
      device_id: device_id,
      device_info: devices[device_id].{
        name,
        location,
        type
      },
      sensor_data: {
        path: sensor_path,
        value: endpoint(sensor_path),
        last_normal_reading: history(sensor_path)
          .reverse()
          .find(record => record.value <= 90)
      }
    }
  })
  
  return {
    alert_time: now(),
    alert_type: "high_temperature",
    affected_devices: at_risk_devices,
    recommended_action: "Investigate cooling systems"
  }
}
```

## Getting Started

### Prerequisites

- Rust 1.70+ and Cargo

### Installation

Clone the repository and build the project:

```bash
git clone https://github.com/yourusername/hyperion.git
cd hyperion
cargo build --release
```

### Basic Usage

```rust
use hyperion::path::Path;
use hyperion::value::Value;
use hyperion::persistent_store::PersistentStore;
use std::str::FromStr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a persistent store
    let db = PersistentStore::open("hyperion_db")?;
    
    // Store data as individual endpoints
    db.set(Path::from_str("users.u-123456.username")?, 
           Value::String("alice".to_string()))?;
           
    db.set(Path::from_str("users.u-123456.email")?, 
           Value::String("alice@example.com".to_string()))?;
    
    // Query data with wildcard pattern
    let email_pattern = Path::from_str("users.*.email")?;
    let results = db.query(&email_pattern)?;
    
    for (path, value) in results {
        println!("{} = {}", path, value);
    }
    
    // Flush changes to disk
    db.flush()?;
    
    Ok(())
}
```

### Using the Query Language

```rust
use hyperion::persistent_store::PersistentStore;
use hyperion::ql;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = PersistentStore::open("hyperion_db")?;
    
    // Execute a query
    let query = r#"{
        users.u-123456.last_login = now()
        return users.u-123456
    }"#;
    
    let result = ql::execute_query(&store, query)?;
    println!("Result: {}", result);
    
    Ok(())
}
```

## Conceptual Architecture

Hyperion is built on several innovative concepts:

1. **Paths as first-class citizens** - The path to data is as important as the data itself
2. **Implicit reconstruction** - Higher-level structures are derived, not stored
3. **Vectorized similarity** - Path relationships are captured in a vector space
4. **Biologically-inspired optimization** - Systems that learn and adapt like living networks
5. **Dynamic locality** - Related data becomes physically co-located over time

## Project Structure

- `src/path.rs` - Path representation and manipulation
- `src/value.rs` - Value types and operations
- `src/entity.rs` - Entity reconstruction from endpoints
- `src/store.rs` - In-memory store implementation
- `src/persistent_store.rs` - Persistent storage with sled
- `src/index.rs` - Indexing capabilities
- `src/wildcard_index.rs` - Specialized index for wildcard queries
- `src/index_batcher.rs` - Performance optimization for index operations
- `src/ql/` - Query language implementation
- `src/bench.rs` - Benchmarking tools

## Roadmap

- [ ] Complete HyperionQL implementation according to the RFC specification
- [ ] Distributed storage and sharding
- [ ] Advanced query optimization
- [ ] Transaction improvements
- [ ] Vectorized path indexing
- [ ] Bio-inspired self-optimization
- [ ] CRUD API and language bindings
- [ ] Admin interface

## Contributing

Hyperion is in its early stages and not yet open for contributions. Star the repository to stay updated on progress.

## License

This project is licensed under [LICENSE] - see the LICENSE file for details.

---

**Note**: Hyperion is experimental research software and not yet ready for production use. Stay tuned for updates as development progresses!