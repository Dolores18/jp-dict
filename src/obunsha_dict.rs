use rusqlite::{Connection, Result, params};
use serde::{Deserialize, Serialize};
use scraper::Html;

/// 旺文社国語辞典词条结构 (Obunsha Kokugo Dictionary Entry)
/// 基于MDX格式的专业日语词典数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObunshaDictEntry {
    pub id: Option<i64>,
    /// 词条ID - 来源于MDX的data-id
    pub data_id: String,
    /// 词条类型 - 来源于MDX的data-type  
    pub data_type: String,
    /// 词条标题 - 从MDX关键词提取的标题
    pub headword: String,
    /// 假名读音 - 提取的假名部分
    pub kana_reading: Option<String>,
    /// 汉字表记 - 提取的汉字部分
    pub kanji_writing: Option<String>,
    /// 词性信息 - 如"自五"等语法信息
    pub part_of_speech: Option<String>,
    /// 活用形 - 动词、形容词的变化形式
    pub conjugation: Option<String>,
    /// 词条定义 - 完整的HTML定义内容
    pub definition_html: String,
    /// 纯文本定义 - 去除HTML标签的纯文本版本
    pub definition_text: String,
    /// 原始MDX内容 - 保留完整的原始数据
    pub raw_mdx_content: String,
}

/// 旺文社国語辞典数据库管理
pub struct ObunshaDictDatabase {
    conn: Connection,
}

impl ObunshaDictDatabase {
    /// 创建新的数据库连接
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        Ok(ObunshaDictDatabase { conn })
    }

    /// 初始化旺文社国語辞典表
    /// 表名: obunsha_kokugo_dict (旺文社国語辞典)
    pub fn initialize(&self) -> Result<()> {
        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS obunsha_kokugo_dict (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                data_id TEXT NOT NULL UNIQUE,               -- MDX词条ID
                data_type TEXT NOT NULL,                    -- MDX词条类型
                headword TEXT NOT NULL,                     -- 词条标题
                kana_reading TEXT,                          -- 假名读音
                kanji_writing TEXT,                         -- 汉字表记
                part_of_speech TEXT,                        -- 词性信息
                conjugation TEXT,                           -- 活用形
                definition_html TEXT NOT NULL,              -- HTML定义
                definition_text TEXT NOT NULL,              -- 纯文本定义
                raw_mdx_content TEXT NOT NULL,              -- 原始MDX内容
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
            [],
        )?;

        // 创建索引以提高查询性能
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_headword ON obunsha_kokugo_dict(headword)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_kana_reading ON obunsha_kokugo_dict(kana_reading)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_data_id ON obunsha_kokugo_dict(data_id)",
            [],
        )?;

        println!("✅ 旺文社国語辞典表已初始化");
        Ok(())
    }

    /// 插入单个词条
    pub fn insert_entry(&self, entry: &ObunshaDictEntry) -> Result<i64> {
        let mut stmt = self.conn.prepare(
            r#"
            INSERT INTO obunsha_kokugo_dict (
                data_id, data_type, headword, kana_reading, kanji_writing,
                part_of_speech, conjugation, definition_html, definition_text, raw_mdx_content
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
        )?;

        let row_id = stmt.insert(params![
            entry.data_id,
            entry.data_type,
            entry.headword,
            entry.kana_reading,
            entry.kanji_writing,
            entry.part_of_speech,
            entry.conjugation,
            entry.definition_html,
            entry.definition_text,
            entry.raw_mdx_content,
        ])?;

        Ok(row_id)
    }

    /// 批量插入词条
    pub fn insert_entries_batch(&self, entries: &[ObunshaDictEntry]) -> Result<usize> {
        let tx = self.conn.unchecked_transaction()?;
        
        {
            let mut stmt = tx.prepare(
                r#"
                INSERT OR REPLACE INTO obunsha_kokugo_dict (
                    data_id, data_type, headword, kana_reading, kanji_writing,
                    part_of_speech, conjugation, definition_html, definition_text, raw_mdx_content
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
            )?;

            for entry in entries {
                stmt.execute(params![
                    entry.data_id,
                    entry.data_type,
                    entry.headword,
                    entry.kana_reading,
                    entry.kanji_writing,
                    entry.part_of_speech,
                    entry.conjugation,
                    entry.definition_html,
                    entry.definition_text,
                    entry.raw_mdx_content,
                ])?;
            }
        }

        tx.commit()?;
        println!("✅ 成功插入 {} 条词条", entries.len());
        Ok(entries.len())
    }

    /// 根据标题查询词条（模糊匹配，保留原有功能）
    pub fn search_by_headword(&self, headword: &str) -> Result<Vec<ObunshaDictEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM obunsha_kokugo_dict WHERE headword LIKE ?1 ORDER BY headword"
        )?;

        let entry_iter = stmt.query_map([format!("%{}%", headword)], |row| {
            Ok(ObunshaDictEntry {
                id: Some(row.get(0)?),
                data_id: row.get(1)?,
                data_type: row.get(2)?,
                headword: row.get(3)?,
                kana_reading: row.get(4)?,
                kanji_writing: row.get(5)?,
                part_of_speech: row.get(6)?,
                conjugation: row.get(7)?,
                definition_html: row.get(8)?,
                definition_text: row.get(9)?,
                raw_mdx_content: row.get(10)?,
            })
        })?;

        let mut entries = Vec::new();
        for entry in entry_iter {
            entries.push(entry?);
        }

        Ok(entries)
    }

    /// 根据假名精确搜索（全等匹配）
    pub fn search_by_kana_exact(&self, kana: &str) -> Result<Vec<ObunshaDictEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM obunsha_kokugo_dict WHERE kana_reading = ?1 ORDER BY headword"
        )?;

        let entry_iter = stmt.query_map([kana], |row| {
            Ok(ObunshaDictEntry {
                id: Some(row.get(0)?),
                data_id: row.get(1)?,
                data_type: row.get(2)?,
                headword: row.get(3)?,
                kana_reading: row.get(4)?,
                kanji_writing: row.get(5)?,
                part_of_speech: row.get(6)?,
                conjugation: row.get(7)?,
                definition_html: row.get(8)?,
                definition_text: row.get(9)?,
                raw_mdx_content: row.get(10)?,
            })
        })?;

        let mut entries = Vec::new();
        for entry in entry_iter {
            entries.push(entry?);
        }

        Ok(entries)
    }

    /// 根据汉字智能搜索（同时进行精确匹配和包含匹配）
    pub fn search_by_kanji_smart(&self, kanji: &str) -> Result<Vec<ObunshaDictEntry>> {
        let mut entries = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        // 首先进行精确匹配
        let mut stmt = self.conn.prepare(
            "SELECT * FROM obunsha_kokugo_dict WHERE kanji_writing = ?1 ORDER BY headword"
        )?;

        let entry_iter = stmt.query_map([kanji], |row| {
            Ok(ObunshaDictEntry {
                id: Some(row.get(0)?),
                data_id: row.get(1)?,
                data_type: row.get(2)?,
                headword: row.get(3)?,
                kana_reading: row.get(4)?,
                kanji_writing: row.get(5)?,
                part_of_speech: row.get(6)?,
                conjugation: row.get(7)?,
                definition_html: row.get(8)?,
                definition_text: row.get(9)?,
                raw_mdx_content: row.get(10)?,
            })
        })?;

        for entry in entry_iter {
            let entry = entry?;
            seen_ids.insert(entry.data_id.clone());
            entries.push(entry);
        }

        // 然后进行LIKE搜索（查找带点号的多重表记）
        let mut stmt = self.conn.prepare(
            "SELECT * FROM obunsha_kokugo_dict WHERE kanji_writing LIKE ?1 ORDER BY headword"
        )?;

        let entry_iter = stmt.query_map([format!("%{}%", kanji)], |row| {
            Ok(ObunshaDictEntry {
                id: Some(row.get(0)?),
                data_id: row.get(1)?,
                data_type: row.get(2)?,
                headword: row.get(3)?,
                kana_reading: row.get(4)?,
                kanji_writing: row.get(5)?,
                part_of_speech: row.get(6)?,
                conjugation: row.get(7)?,
                definition_html: row.get(8)?,
                definition_text: row.get(9)?,
                raw_mdx_content: row.get(10)?,
            })
        })?;

        for entry_result in entry_iter {
            let entry = entry_result?;
            // 避免重复添加已经在精确匹配中找到的词条
            if !seen_ids.contains(&entry.data_id) {
                // 应用层过滤：检查是否真的匹配（支持点号分割的多重表记）
                if let Some(ref kanji_writing) = entry.kanji_writing {
                    if kanji_writing.split('·').any(|part| part == kanji) {
                        entries.push(entry);
                    }
                }
            }
        }

        Ok(entries)
    }

    /// 获取表的统计信息
    pub fn get_stats(&self) -> Result<(i64, i64)> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM obunsha_kokugo_dict",
            [],
            |row| row.get(0)
        )?;

        let unique_headwords: i64 = self.conn.query_row(
            "SELECT COUNT(DISTINCT headword) FROM obunsha_kokugo_dict",
            [],
            |row| row.get(0)
        )?;

        Ok((count, unique_headwords))
    }

    /// 从清理后的数据文件解析并导入所有词条
    pub fn import_from_cleaned_data(&self, cleaned_data_path: &str) -> Result<usize, Box<dyn std::error::Error>> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        println!("🚀 开始从清理数据导入词条: {}", cleaned_data_path);

        let file = File::open(cleaned_data_path)?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        let mut entries = Vec::new();
        let mut current_title: Option<String> = None;
        let mut processed_count = 0;

        while let Some(line_result) = lines.next() {
            let line = line_result?;
            
            if line.trim().is_empty() {
                // 空行表示词条结束，重置状态
                current_title = None;
                continue;
            }

            if line.contains("<link rel=\"stylesheet\"") {
                // 这是HTML内容行
                if let Some(title) = current_title.take() {
                    // 解析这个词条
                    if let Some(entry) = self.parse_entry_from_html(&title, &line) {
                        entries.push(entry);
                        processed_count += 1;

                        // 每1000条批量插入一次
                        if entries.len() >= 1000 {
                            self.insert_entries_batch(&entries)?;
                            entries.clear();
                            println!("✅ 已导入 {} 条词条", processed_count);
                        }
                    }
                }
            } else {
                // 这是标题行
                current_title = Some(line);
            }
        }

        // 插入剩余的词条
        if !entries.is_empty() {
            self.insert_entries_batch(&entries)?;
        }

        println!("🎉 导入完成！共处理 {} 条词条", processed_count);
        Ok(processed_count)
    }

    /// 从HTML解析单个词条
    fn parse_entry_from_html(&self, title: &str, html: &str) -> Option<ObunshaDictEntry> {
        use scraper::{Html, Selector};

        let document = Html::parse_fragment(html);
        
        // 提取data-id
        let container_selector = Selector::parse("container").ok()?;
        let container = document.select(&container_selector).next()?;
        let data_id = container.value().attr("data-id")?.to_string();
        let data_type = container.value().attr("data-type").unwrap_or("unknown").to_string();

        // CSS选择器
        let kana_selector = Selector::parse(".headword_kana").ok()?;
        let kanji_selector = Selector::parse(".headword_hyouki").ok()?;
        let ryaku_selector = Selector::parse(".headword_ryaku").ok()?;
        let pos_selector = Selector::parse(".pos_s").ok()?;
        let katsuyo_selector = Selector::parse(".katsuyo").ok()?;

        let mut kana_reading: Option<String> = None;
        let mut kanji_writing: Option<String> = None;
        let mut part_of_speech: Option<String> = None;
        let mut conjugation: Option<String> = None;

        // 优先从headline（title）解析假名和汉字
        if let Some((kana, kanji)) = self.parse_headline(title) {
            kana_reading = Some(kana);
            kanji_writing = Some(kanji);
        } else {
        }

        // 如果从headline解析失败，再从HTML中选择器提取
        if kana_reading.is_none() {
            if let Some(kana_element) = document.select(&kana_selector).next() {
                let kana_text = kana_element.text().collect::<String>();
                let cleaned_kana = self.clean_kana_text(&kana_text);
                if !cleaned_kana.is_empty() {
                    kana_reading = Some(cleaned_kana);
                }
            }
        }

        if kanji_writing.is_none() {
            if let Some(kanji_element) = document.select(&kanji_selector).next() {
                let kanji_text = kanji_element.text().collect::<String>();
                let cleaned_kanji = self.clean_kanji_text(&kanji_text);
                if !cleaned_kanji.is_empty() {
                    kanji_writing = Some(cleaned_kanji);
                }
            }
        }

        // 对于英文缩写词条，提取ryaku作为假名读音
        if kana_reading.is_none() {
            if let Some(ryaku_element) = document.select(&ryaku_selector).next() {
                let ryaku_text = ryaku_element.text().collect::<String>();
                let cleaned_ryaku = self.clean_kana_text(&ryaku_text);
                if !cleaned_ryaku.is_empty() {
                    kana_reading = Some(cleaned_ryaku);
                }
            }
        }

        // 提取词性信息
        if let Some(pos_element) = document.select(&pos_selector).next() {
            let pos_text = pos_element.text().collect::<String>().trim().to_string();
            if !pos_text.is_empty() {
                part_of_speech = Some(pos_text);
            }
        }

        // 提取活用形
        if let Some(katsuyo_element) = document.select(&katsuyo_selector).next() {
            let katsuyo_text = katsuyo_element.text().collect::<String>().trim().to_string();
            if !katsuyo_text.is_empty() {
                conjugation = Some(katsuyo_text);
            }
        }

        // 提取纯文本定义
        let definition_text = self.extract_definition_text(&document);

        Some(ObunshaDictEntry {
            id: None,
            data_id,
            data_type,
            headword: title.to_string(),
            kana_reading,
            kanji_writing,
            part_of_speech,
            conjugation,
            definition_html: html.to_string(),
            definition_text,
            raw_mdx_content: format!("{}\n{}", title, html),
        })
    }

    /// 从headline解析假名和汉字
    fn parse_headline(&self, headline: &str) -> Option<(String, String)> {
        let headline = headline.trim();
        
        // 检查是否包含【】括号格式：假名【汉字】
        if let Some(start) = headline.find('【') {
            if let Some(end) = headline.find('】') {
                if start < end {
                    // 使用chars()迭代器来正确处理中文字符
                    let chars: Vec<char> = headline.chars().collect();
                    
                    // 将字节索引转换为字符索引
                    let start_char = headline[..start].chars().count();
                    let end_char = headline[..end].chars().count();
                    
                    if start_char < end_char && start_char < chars.len() && end_char < chars.len() {
                        let kana_part: String = chars[..start_char].iter().collect();
                        let kanji_part: String = chars[start_char + 1..end_char].iter().collect();
                        
                        // 假名部分不能为空，汉字部分可以为空（如：ば【】）
                        if !kana_part.is_empty() {
                            return Some((kana_part, kanji_part));
                        }
                    }
                }
            }
        }
        
        // 如果没有括号，检查是否只有假名
        if !headline.is_empty() {
            // 检查是否包含汉字
            let has_kanji = headline.chars().any(|c| {
                c >= '\u{4e00}' && c <= '\u{9fff}' // CJK统一汉字
            });
            
            if !has_kanji {
                // 只有假名的情况
                return Some((headline.to_string(), String::new()));
            }
        }
        
        None
    }

    /// 清理假名文本，去除特殊符号和HTML标签
    fn clean_kana_text(&self, text: &str) -> String {
        let mut result = String::new();
        
        for ch in text.chars() {
            match ch {
                // 保留平假名
                '\u{3040}'..='\u{309f}' => result.push(ch),
                // 保留片假名
                '\u{30a0}'..='\u{30ff}' => result.push(ch),
                // 保留片假名长音符号
                'ー' => result.push(ch),
                // 保留英文和数字（用于英文缩写词条）
                _ if ch.is_ascii_alphanumeric() => result.push(ch),
                // 对于英文词条，保留连字符和下划线
                '-' | '_' if text.chars().any(|c| c.is_ascii_alphabetic()) => result.push(ch),
                // 过滤掉所有其他符号，包括日语词条中的ASCII连字符
                _ => {}
            }
        }
        
        result.trim().to_string()
    }

    /// 清理汉字文本，去除标记符号
    fn clean_kanji_text(&self, text: &str) -> String {
        let mut result = String::new();
        
        for ch in text.chars() {
            match ch {
                // 保留汉字 (CJK统一汉字)
                '\u{4e00}'..='\u{9fff}' => result.push(ch),
                // 保留平假名
                '\u{3040}'..='\u{309f}' => result.push(ch),
                // 保留片假名
                '\u{30a0}'..='\u{30ff}' => result.push(ch),
                // 保留一些基本符号
                '・' | '‧' | '·' | '-' | 'ー' => result.push(ch),
                // 过滤掉标记符号
                '【' | '】' | '◇' | '△' | '▽' | '▲' | '▼' | '○' | '●' | '◯' | 
                '□' | '■' | '▢' | '▣' | '◆' | '※' | '＊' | '☆' | '★' => {
                    // 跳过这些标记符号
                },
                // 保留其他可能有用的字符（如英文、数字）
                _ if ch.is_alphanumeric() => result.push(ch),
                _ => {} // 跳过其他特殊符号
            }
        }
        
        result.trim().to_string()
    }

    /// 提取定义的纯文本内容
    fn extract_definition_text(&self, document: &Html) -> String {
        use scraper::Selector;

        let meaning_selectors = [
            ".mean_normal",
            ".mean_lv_2", 
            ".mean_lv_1",
            ".mean_no_1",
            ".mean_no_2", 
            ".mean_no_3",
        ];
        
        let mut meanings = Vec::new();
        
        for selector_str in &meaning_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    let text = element.text().collect::<Vec<_>>().join("");
                    let cleaned_text = text.trim();
                    if !cleaned_text.is_empty() {
                        meanings.push(cleaned_text.to_string());
                    }
                }
            }
        }
        
        if meanings.is_empty() {
            // 如果没有找到特定的释义元素，提取所有文本
            document.root_element().text().collect::<String>().trim().to_string()
        } else {
            meanings.join(" ")
        }
    }
} 