# Multi-Agent System Demo with Ollama and Redis

这个示例展示了如何使用自定义的Ollama服务器和Redis内存来运行多代理系统。

## 🔧 配置

### LLM 配置
- **服务器**: 192.168.1.38:11434
- **模型**: qwen3:4b-thinking-2507-q8_0
- **协议**: OpenAI兼容API

### 内存配置
- **Redis服务器**: 172.16.0.127:6379
- **键前缀**: 根据代理类型自动设置 (team_agent, human_agent, hybrid_agent)

## 🚀 运行示例

### 前置条件

1. **Ollama服务器运行中**
   ```bash
   # 确保Ollama服务器在192.168.1.38:11434运行
   # 并且已经拉取了qwen3:4b-thinking-2507-q8_0模型
   ```

2. **Redis服务器运行中**
   ```bash
   # 确保Redis服务器在172.16.0.127:6379运行
   redis-cli -h 172.16.0.127 -p 6379 ping
   ```

3. **Rust依赖** (可选，用于真实Redis支持)
   ```toml
   # 在Cargo.toml中添加以下依赖以获得真实Redis支持
   [dependencies]
   redis = "0.24"
   ```

### 运行示例

```bash
# 运行多代理系统演示
cargo run --example multi_agent_system

# 如果需要启用特定功能
cargo run --example multi_agent_system --features ollama
```

## 📋 演示内容

### 1. Sequential Team Agent with Redis Memory
- 使用Redis存储团队协调信息
- 数学代理 → 数据分析代理的顺序执行
- 共享内存上下文

### 2. Concurrent Team Agent
- 并发执行多个代理
- 结果聚合

### 3. Hybrid Execution Pattern
- 复杂的执行模式：系统代理 → (数学代理 || 数据代理) → 协调代理
- 混合并发和顺序执行

### 4. Human Agent with Redis Memory
- 基于关键词的人工干预
- Redis存储交互历史
- 模拟人工输入（演示模式）

### 5. Team-Human Hybrid with Shared Redis Memory
- 团队执行与人工监督结合
- 共享Redis内存
- 复杂的干预逻辑

### 6. Agent Registry and Universal Tools
- 代理注册表管理
- 代理作为工具使用
- 元代理委派任务

### 7. Nested Team Agents
- 嵌套团队结构
- 分析团队 + 运维团队 → 主协调员
- 层次化代理组织

## 🔍 Redis内存实现

当前示例包含一个模拟的Redis内存实现用于演示。在生产环境中，你需要：

1. **添加Redis依赖**
   ```toml
   redis = "0.24"
   tokio = { version = "1", features = ["full"] }
   ```

2. **实现真实的Redis客户端**
   ```rust
   use redis::AsyncCommands;
   
   impl BaseMemory for RedisMemory {
       fn messages(&self) -> Vec<Message> {
           // 使用Redis客户端获取消息
           // 注意：BaseMemory trait是同步的，你可能需要
           // 使用tokio::task::block_in_place或重新设计为异步
       }
       
       fn add_message(&mut self, message: Message) {
           // 使用Redis客户端存储消息
       }
   }
   ```

## 🎯 关键特性

- ✅ **Ollama集成**: 使用自定义Ollama服务器和模型
- ✅ **Redis内存**: 持久化对话历史和代理状态
- ✅ **多执行模式**: 并发、顺序、混合执行模式
- ✅ **人工干预**: 可配置的人工监督和干预
- ✅ **嵌套团队**: 复杂的层次化代理结构
- ✅ **工具集成**: 代理作为工具的通用集成
- ✅ **内存共享**: 跨代理的上下文共享

## 🔧 自定义配置

你可以通过修改以下部分来自定义配置：

```rust
// 修改Ollama配置
let ollama_config = OllamaConfig::new()
    .with_api_base("http://YOUR_OLLAMA_SERVER:11434/v1");

let llm = OpenAI::new(ollama_config)
    .with_model("YOUR_MODEL_NAME");

// 修改Redis配置
let redis_memory = RedisMemory::new("redis://YOUR_REDIS_SERVER:6379", "your_prefix")
    .expect("Failed to connect to Redis");
```

## 📝 注意事项

1. **网络连接**: 确保能够访问指定的Ollama和Redis服务器
2. **模型可用性**: 确保Ollama服务器已经拉取了指定的模型
3. **内存实现**: 当前使用模拟Redis实现，生产环境需要真实Redis客户端
4. **错误处理**: 示例包含基本错误处理，生产环境可能需要更robust的错误处理

## 🚀 扩展建议

1. **真实Redis集成**: 实现完整的异步Redis客户端
2. **配置文件**: 使用配置文件管理服务器地址和模型名称
3. **监控和日志**: 添加详细的监控和日志记录
4. **错误恢复**: 实现网络错误和服务不可用时的恢复机制
5. **性能优化**: 添加连接池和缓存机制
