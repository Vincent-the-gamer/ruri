use crate::conversation::models::{
    AddMessageRequest, ChatType, Conversation, ConversationFilter, CreateConversationRequest,
    Message,
};
use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::{Row, SqlitePool};
use tracing::{debug, error, info};

/// 对话数据库管理器
pub struct ConversationDatabase {
    pool: SqlitePool,
}

impl ConversationDatabase {
    /// Create a new ConversationDatabase using a **shared** SqlitePool.
    ///
    /// The pool is typically obtained from `crate::db::init()` so that all
    /// features (conversations, MCP, …) share the same `ruri.db`.
    ///
    /// Schema creation is **not** performed here – it is handled centrally by
    /// `crate::db::init_schema()`.  This constructor only runs a lightweight
    /// check to make sure the required tables exist.
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        let db = Self { pool };

        // Quick sanity-check: ensure the conversations table is reachable.
        // If the shared `db::init()` has already run this will be a no-op.
        sqlx::query("SELECT 1 FROM conversations LIMIT 1")
            .execute(&db.pool)
            .await
            .context("Conversations table not found – did db::init() run?")?;

        info!("ConversationDatabase ready (shared pool)");
        Ok(db)
    }

    /// 创建新会话
    pub async fn create_conversation(
        &self,
        req: CreateConversationRequest,
    ) -> Result<Conversation> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        let chat_type_str = match req.chat_type {
            ChatType::Group => "group",
            ChatType::Private => "private",
        };

        sqlx::query(
            r#"
            INSERT INTO conversations (id, bot_name, chat_type, chat_id, title, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
            "#,
        )
        .bind(&id)
        .bind(&req.bot_name)
        .bind(chat_type_str)
        .bind(&req.chat_id)
        .bind(&req.title)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create conversation")?;

        debug!("Created conversation with id: {}", id);

        // 查询并返回创建的会话
        self.get_conversation_by_id(&id).await
    }

    /// 根据ID获取会话
    pub async fn get_conversation_by_id(&self, id: &str) -> Result<Conversation> {
        let row = sqlx::query(
            r#"
            SELECT id, bot_name, chat_type, chat_id, title, created_at, updated_at
            FROM conversations
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to get conversation")?;

        let chat_type_str: String = row.try_get("chat_type")?;
        let chat_type = match chat_type_str.as_str() {
            "group" => ChatType::Group,
            "private" => ChatType::Private,
            _ => ChatType::Private, // 默认值
        };

        Ok(Conversation {
            id: row.try_get("id")?,
            bot_name: row.try_get("bot_name")?,
            chat_type,
            chat_id: row.try_get("chat_id")?,
            title: row.try_get("title").ok(),
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    /// 查询会话列表
    pub async fn list_conversations(
        &self,
        filter: Option<ConversationFilter>,
    ) -> Result<Vec<Conversation>> {
        let mut query = String::from(
            r#"
            SELECT id, bot_name, chat_type, chat_id, title, created_at, updated_at
            FROM conversations
            WHERE 1=1
            "#,
        );

        #[allow(unused_assignments)]
        let mut bind_index = 1;

        #[allow(unused_assignments)]
        if let Some(ref f) = filter {
            if let Some(ref _bot_name) = f.bot_name {
                query.push_str(&format!(" AND bot_name = ?{}", bind_index));
                bind_index += 1;
            }
            if let Some(chat_type) = f.chat_type {
                let _chat_type_str = match chat_type {
                    ChatType::Group => "group",
                    ChatType::Private => "private",
                };
                query.push_str(&format!(" AND chat_type = ?{}", bind_index));
                bind_index += 1;
            }
            if let Some(ref _keyword) = f.keyword {
                query.push_str(&format!(
                    " AND (title LIKE ?{} OR chat_id LIKE ?{})",
                    bind_index,
                    bind_index + 1
                ));
                bind_index += 2;
            }
        }

        query.push_str(" ORDER BY updated_at DESC");

        // 将所有绑定的值移到外面，确保在查询执行期间有效
        let bot_name_bind: Option<String> = filter.as_ref().and_then(|f| f.bot_name.clone());
        let chat_type_bind: Option<String> = filter.as_ref().and_then(|f| {
            f.chat_type.map(|ct| match ct {
                ChatType::Group => "group".to_string(),
                ChatType::Private => "private".to_string(),
            })
        });
        let keyword_bind: Option<String> = filter.as_ref().and_then(|f| f.keyword.clone());
        let keyword_pattern_1: Option<String> = keyword_bind.as_ref().map(|k| format!("%{}%", k));
        let keyword_pattern_2: Option<String> = keyword_bind.as_ref().map(|k| format!("%{}%", k));

        let mut sql_query = sqlx::query(&query);

        if let Some(ref bot_name) = bot_name_bind {
            sql_query = sql_query.bind(bot_name);
        }
        if let Some(ref chat_type) = chat_type_bind {
            sql_query = sql_query.bind(chat_type);
        }
        if let (Some(pattern1), Some(pattern2)) = (&keyword_pattern_1, &keyword_pattern_2) {
            sql_query = sql_query.bind(pattern1).bind(pattern2);
        }

        let rows = sql_query
            .fetch_all(&self.pool)
            .await
            .context("Failed to list conversations")?;

        let conversations: Result<Vec<Conversation>, sqlx::Error> = rows
            .iter()
            .map(|row| {
                let chat_type_str: String = row.try_get("chat_type")?;
                let chat_type = match chat_type_str.as_str() {
                    "group" => ChatType::Group,
                    "private" => ChatType::Private,
                    _ => ChatType::Private,
                };

                Ok(Conversation {
                    id: row.try_get("id")?,
                    bot_name: row.try_get("bot_name")?,
                    chat_type,
                    chat_id: row.try_get("chat_id")?,
                    title: row.try_get("title").ok(),
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                })
            })
            .collect();

        Ok(conversations.context("Failed to parse conversation rows")?)
    }

    /// 添加消息到会话
    pub async fn add_message(&self, req: AddMessageRequest) -> Result<Message> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO messages (id, conversation_id, role, content, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
        )
        .bind(&id)
        .bind(&req.conversation_id)
        .bind(&req.role)
        .bind(&req.content)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to add message")?;

        // 更新会话的 updated_at 时间
        sqlx::query(
            r#"
            UPDATE conversations
            SET updated_at = ?1
            WHERE id = ?2
            "#,
        )
        .bind(now)
        .bind(&req.conversation_id)
        .execute(&self.pool)
        .await
        .context("Failed to update conversation timestamp")?;

        debug!(
            "Added message with id: {} to conversation: {}",
            id, req.conversation_id
        );

        // 查询并返回添加的消息
        self.get_message_by_id(&id).await
    }

    /// 根据ID获取消息
    pub async fn get_message_by_id(&self, id: &str) -> Result<Message> {
        let row = sqlx::query(
            r#"
            SELECT id, conversation_id, role, content, created_at
            FROM messages
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to get message")?;

        Ok(Message {
            id: row.try_get("id")?,
            conversation_id: row.try_get("conversation_id")?,
            role: row.try_get("role")?,
            content: row.try_get("content")?,
            created_at: row.try_get("created_at")?,
        })
    }

    /// 获取会话的所有消息
    pub async fn get_conversation_messages(&self, conversation_id: &str) -> Result<Vec<Message>> {
        let rows = sqlx::query(
            r#"
            SELECT id, conversation_id, role, content, created_at
            FROM messages
            WHERE conversation_id = ?1
            ORDER BY created_at ASC
            "#,
        )
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to get conversation messages")?;

        let messages: Result<Vec<Message>, sqlx::Error> = rows
            .iter()
            .map(|row| {
                Ok(Message {
                    id: row.try_get("id")?,
                    conversation_id: row.try_get("conversation_id")?,
                    role: row.try_get("role")?,
                    content: row.try_get("content")?,
                    created_at: row.try_get("created_at")?,
                })
            })
            .collect();

        Ok(messages.context("Failed to parse message rows")?)
    }

    /// 删除会话（级联删除相关消息）
    pub async fn delete_conversation(&self, id: &str) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM conversations
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to delete conversation")?;

        debug!("Deleted conversation with id: {}", id);
        Ok(())
    }

    /// 根据bot_name、chat_type和chat_id查找或创建会话
    pub async fn get_or_create_conversation(
        &self,
        bot_name: String,
        chat_type: ChatType,
        chat_id: String,
    ) -> Result<Conversation> {
        let chat_type_str = match chat_type {
            ChatType::Group => "group",
            ChatType::Private => "private",
        };

        // 尝试查找现有会话
        match sqlx::query(
            r#"
            SELECT id, bot_name, chat_type, chat_id, title, created_at, updated_at
            FROM conversations
            WHERE bot_name = ?1 AND chat_type = ?2 AND chat_id = ?3
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(&bot_name)
        .bind(chat_type_str)
        .bind(&chat_id)
        .fetch_optional(&self.pool)
        .await
        {
            Ok(Some(row)) => {
                let chat_type_str: String = row.try_get("chat_type")?;
                let chat_type = match chat_type_str.as_str() {
                    "group" => ChatType::Group,
                    "private" => ChatType::Private,
                    _ => ChatType::Private,
                };

                Ok(Conversation {
                    id: row.try_get("id")?,
                    bot_name: row.try_get("bot_name")?,
                    chat_type,
                    chat_id: row.try_get("chat_id")?,
                    title: row.try_get("title").ok(),
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                })
            }
            Ok(None) => {
                // 不存在则创建新会话
                self.create_conversation(CreateConversationRequest {
                    bot_name,
                    chat_type,
                    chat_id,
                    title: None,
                })
                .await
            }
            Err(e) => {
                error!("Failed to query conversation: {}", e);
                Err(e).context("Failed to query conversation")
            }
        }
    }
}
