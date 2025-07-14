use rusqlite::{Connection, Result, params};
use serde::{Deserialize, Serialize};

/// 表現読解国語辞典 - 日语词典条目结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionaryEntry {
    pub id: Option<i64>,
    /// 假名entry - 词条的假名读音
    pub kana_entry: String,
    /// 汉字格式 - 汉字表记（可能为空，如纯假名词汇）
    pub kanji_form: Option<String>,
    /// 释义字段 - 词汇的含义解释
    pub meaning: String,
    /// 发音字段 - 发音信息（音读、训读等）
    pub pronunciation: Option<String>,
    /// 词条类型 - 如汉字条目、一般条目等
    pub entry_type: String,
    /// 原始HTML内容 - 保留原始数据用于调试
    pub raw_html: String,
}

/// 数据库管理结构
pub struct Database {
    conn: Connection,
}

impl Database {
    /// 创建新的数据库连接
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        Ok(Database { conn })
    }

    /// 初始化数据库表 - 表現読解国語辞典
    pub fn initialize(&self) -> Result<()> {
        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS dictionary_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                kana_entry TEXT NOT NULL,                    -- 假名entry
                kanji_form TEXT,                            -- 汉字格式
                meaning TEXT NOT NULL,                      -- 释义字段  
                pronunciation TEXT,                         -- 发音字段
                entry_type TEXT NOT NULL,                   -- 词条类型
                raw_html TEXT NOT NULL,                     -- 原始HTML
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
            [],
        )?;

        // 创建索引以提高查询性能
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_kana_entry ON dictionary_entries(kana_entry)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_kanji_form ON dictionary_entries(kanji_form)",
            [],
        )?;

        Ok(())
    }

    /// 插入词典条目
    pub fn insert_entry(&self, entry: &DictionaryEntry) -> Result<i64> {
        let mut stmt = self.conn.prepare(
            r#"
            INSERT INTO dictionary_entries 
            (kana_entry, kanji_form, meaning, pronunciation, entry_type, raw_html)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )?;

        stmt.execute(params![
            entry.kana_entry,
            entry.kanji_form,
            entry.meaning,
            entry.pronunciation,
            entry.entry_type,
            entry.raw_html,
        ])?;

        Ok(self.conn.last_insert_rowid())
    }

    /// 批量插入词典条目
    pub fn insert_entries_batch(&self, entries: &[DictionaryEntry]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        
        {
            let mut stmt = tx.prepare(
                r#"
                INSERT INTO dictionary_entries 
                (kana_entry, kanji_form, meaning, pronunciation, entry_type, raw_html)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
            )?;

            for entry in entries {
                stmt.execute(params![
                    entry.kana_entry,
                    entry.kanji_form,
                    entry.meaning,
                    entry.pronunciation,
                    entry.entry_type,
                    entry.raw_html,
                ])?;
            }
        } // stmt在这里被丢弃

        tx.commit()?;
        Ok(())
    }

    /// 根据假名查询词条
    pub fn find_by_kana(&self, kana: &str) -> Result<Vec<DictionaryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kana_entry, kanji_form, meaning, pronunciation, entry_type, raw_html 
             FROM dictionary_entries WHERE kana_entry = ?1"
        )?;

        let entry_iter = stmt.query_map([kana], |row| {
            Ok(DictionaryEntry {
                id: Some(row.get(0)?),
                kana_entry: row.get(1)?,
                kanji_form: row.get(2)?,
                meaning: row.get(3)?,
                pronunciation: row.get(4)?,
                entry_type: row.get(5)?,
                raw_html: row.get(6)?,
            })
        })?;

        let mut entries = Vec::new();
        for entry in entry_iter {
            entries.push(entry?);
        }
        Ok(entries)
    }

    /// 获取词条总数
    pub fn get_entry_count(&self) -> Result<i32> {
        let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM dictionary_entries")?;
        let count: i32 = stmt.query_row([], |row| row.get(0))?;
        Ok(count)
    }

    /// 清空词条表 - 用于重新导入数据
    pub fn clear_all_entries(&self) -> Result<()> {
        self.conn.execute("DELETE FROM dictionary_entries", [])?;
        // 重置自增ID计数器
        self.conn.execute("DELETE FROM sqlite_sequence WHERE name='dictionary_entries'", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_creation() {
        let db = Database::new(":memory:").unwrap();
        db.initialize().unwrap();
        
        let count = db.get_entry_count().unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_entry_insertion() {
        let db = Database::new(":memory:").unwrap();
        db.initialize().unwrap();

        let entry = DictionaryEntry {
            id: None,
            kana_entry: "あい".to_string(),
            kanji_form: Some("愛".to_string()),
            meaning: "かわいがりいつくしむ気持ち".to_string(),
            pronunciation: Some("アイ".to_string()),
            entry_type: "item_kiso".to_string(),
            raw_html: "<div>test</div>".to_string(),
        };

        let id = db.insert_entry(&entry).unwrap();
        assert!(id > 0);

        let count = db.get_entry_count().unwrap();
        assert_eq!(count, 1);
    }
} 