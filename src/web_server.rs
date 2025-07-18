use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

use crate::obunsha_dict::{ObunshaDictDatabase, ObunshaDictEntry};

/// 查询请求参数
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    /// 查询的单词
    pub word: String,
    /// 查询类型：exact(精确匹配), fuzzy(模糊匹配), kana(假名匹配), kanji(汉字匹配)
    #[serde(default = "default_search_type")]
    pub search_type: String,
}

fn default_search_type() -> String {
    "exact".to_string()
}

/// API响应结构
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    /// 是否成功
    pub success: bool,
    /// 返回的词条数量
    pub count: usize,
    /// 词条列表
    pub entries: Vec<ObunshaDictEntry>,
    /// 查询信息
    pub query_info: QueryInfo,
}

/// 查询信息
#[derive(Debug, Serialize)]
pub struct QueryInfo {
    /// 查询的单词
    pub word: String,
    /// 查询类型
    pub search_type: String,
    /// 查询耗时(毫秒)
    pub duration_ms: u128,
}

/// 错误响应
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

/// 应用状态 - 使用数据库路径而非直接共享连接
#[derive(Clone)]
pub struct AppState {
    pub db_path: String,
}

/// 启动Web服务器
pub async fn start_server(db_path: &str, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 正在启动旺文社词典API服务器...");
    
    let app_state = AppState {
        db_path: db_path.to_string(),
    };

    // 构建路由
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/search", get(search_handler))
        .route("/stats", get(stats_handler))
        .with_state(app_state);

    // 绑定端口并启动服务器
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    
    println!("✅ 服务器已启动！");
    println!("📡 API地址: http://localhost:{}", port);
    println!("🔍 查询接口: http://localhost:{}/search?word=単語", port);
    println!("📊 统计接口: http://localhost:{}/stats", port);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

/// 根路径处理器
async fn root_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "service": "旺文社国語辞典 API",
        "version": "1.0.0",
        "description": "日语词典查询API服务",
        "endpoints": {
            "/": "服务信息",
            "/search": "词条查询 (参数: word, search_type)",
            "/stats": "数据库统计信息"
        },
        "search_types": [
            "exact",
            "fuzzy", 
            "kana",
            "kanji"
        ],
        "example": "/search?word=愛&search_type=fuzzy"
    }))
}

/// 查询处理器
async fn search_handler(
    Query(params): Query<SearchQuery>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let start_time = std::time::Instant::now();

    // 验证查询参数
    if params.word.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                success: false,
                error: "查询词不能为空".to_string(),
            }),
        ));
    }

    // 在新线程中执行数据库查询
    let db_path = state.db_path.clone();
    let search_word = params.word.clone();
    let search_type = params.search_type.clone();

    let result = tokio::task::spawn_blocking(move || {
        let db = ObunshaDictDatabase::new(&db_path)?;
        
        // 使用改进的搜索逻辑
        let entries = match search_type.as_str() {
            "exact" => {
                // 先尝试假名精确搜索
                let mut results = db.search_by_kana_exact(&search_word)?;
                if results.is_empty() {
                    // 如果假名搜索无结果，尝试汉字智能搜索
                    results = db.search_by_kanji_smart(&search_word)?;
                }
                results
            },
            "kana" => db.search_by_kana_exact(&search_word)?,
            "kanji" => db.search_by_kanji_smart(&search_word)?,
            "fuzzy" | _ => db.search_by_headword(&search_word)?,
        };

        Ok::<Vec<ObunshaDictEntry>, Box<dyn std::error::Error + Send + Sync>>(entries)
    }).await;

    let entries = match result {
        Ok(Ok(entries)) => entries,
        Ok(Err(e)) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    success: false,
                    error: format!("数据库查询失败: {}", e),
                }),
            ));
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    success: false,
                    error: format!("查询任务失败: {}", e),
                }),
            ));
        }
    };

    let duration = start_time.elapsed();

    Ok(Json(serde_json::json!({
        "success": true,
        "count": entries.len(),
        "entries": entries,
        "query_info": {
            "word": params.word,
            "search_type": params.search_type,
            "duration_ms": duration.as_millis()
        }
    })))
}

/// 统计信息处理器
async fn stats_handler(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let db_path = state.db_path.clone();
    
    let result = tokio::task::spawn_blocking(move || {
        let db = ObunshaDictDatabase::new(&db_path)?;
        let (count, unique_headwords) = db.get_stats()?;
        Ok::<(i64, i64), Box<dyn std::error::Error + Send + Sync>>((count, unique_headwords))
    }).await;

    let (count, unique_headwords) = match result {
        Ok(Ok(stats)) => stats,
        Ok(Err(e)) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    success: false,
                    error: format!("获取统计信息失败: {}", e),
                }),
            ));
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    success: false,
                    error: format!("统计任务失败: {}", e),
                }),
            ));
        }
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "database": {
            "path": state.db_path,
            "total_entries": count,
            "unique_headwords": unique_headwords,
            "status": "已连接"
        },
        "api": {
            "version": "1.0.0",
            "supported_search_types": ["exact", "fuzzy", "kana", "kanji"]
        }
    })))
}