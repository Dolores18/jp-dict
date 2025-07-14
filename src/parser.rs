use crate::database::DictionaryEntry;
use regex::Regex;
use scraper::{Html, Selector};
use std::fs::File;
use std::io::{BufRead, BufReader};

/// HTML解析器 - 用于提取jpdict.txt中的词典数据
pub struct DictParser {
    /// 清理假名键值的正则表达式
    kana_cleaner: Regex,
    /// 清理汉字的正则表达式  
    kanji_cleaner: Regex,
    /// 提取发音的正则表达式
    pronunciation_extractor: Regex,
}

impl DictParser {
    /// 创建新的解析器
    pub fn new() -> Self {
        Self {
            // 清理假名中的标点符号：点号、中划线、空格等
            kana_cleaner: Regex::new(r"[・\-\s]+").unwrap(),
            // 清理汉字中的括号和标记符号
            kanji_cleaner: Regex::new(r"[【】〔〕（）\(\)〖〗]").unwrap(),
            // 提取粗体发音标记
            pronunciation_extractor: Regex::new(r"<b>([^<]+)</b>").unwrap(),
        }
    }

    /// 清理假名键值 - 去除标点符号
    fn clean_kana(&self, kana: &str) -> String {
        let cleaned = self.kana_cleaner.replace_all(kana.trim(), "");
        cleaned.to_string()
    }

    /// 清理汉字 - 去除括号等符号
    fn clean_kanji(&self, kanji: &str) -> Option<String> {
        // 先去除括号和装饰符号
        let cleaned = self.kanji_cleaner.replace_all(kanji.trim(), "");
        // 去除上标符号和其他装饰符号
        let cleaned = cleaned.replace("◇", "").replace("△", "").replace("▽", "")
                           .replace("▲", "").replace("◆", "").replace("■", "")
                           .replace("●", "").replace("○", "").replace("□", "")
                           .replace("◎", "").replace("※", "").replace("★", "")
                           .replace("☆", "").replace("♦", "").replace("♠", "")
                           .replace("♣", "").replace("♥", "");
        
        if cleaned.trim().is_empty() {
            None
        } else {
            Some(cleaned.trim().to_string())
        }
    }

    /// 提取发音信息
    fn extract_pronunciation(&self, html: &str) -> Option<String> {
        let mut pronunciations = Vec::new();
        
        for cap in self.pronunciation_extractor.captures_iter(html) {
            if let Some(pronunciation) = cap.get(1) {
                pronunciations.push(pronunciation.as_str());
            }
        }
        
        if pronunciations.is_empty() {
            None
        } else {
            Some(pronunciations.join("・"))
        }
    }

    /// 提取释义文本 - 去除HTML标签，保留文本内容
    fn extract_meaning(&self, html: &str) -> String {
        let document = Html::parse_fragment(html);
        
        // 选择释义相关的元素
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
                    if !text.trim().is_empty() {
                        meanings.push(text.trim().to_string());
                    }
                }
            }
        }
        
        if meanings.is_empty() {
            // 如果没有找到特定的释义元素，提取整个文档的文本
            let root_selector = Selector::parse("*").unwrap();
            let all_text: String = document.select(&root_selector)
                .map(|element| element.text().collect::<String>())
                .collect::<Vec<_>>()
                .join(" ");
            all_text.trim().to_string()
        } else {
            meanings.join(" ")
        }
    }

    /// 解析单个词条的HTML
    pub fn parse_entry(&self, html_content: &str) -> Option<DictionaryEntry> {
        let document = Html::parse_fragment(html_content);
        
        // 提取假名
        let kana_selector = Selector::parse(".head_kana").ok()?;
        let kana_element = document.select(&kana_selector).next()?;
        let raw_kana = kana_element.text().collect::<String>();
        let cleaned_kana = self.clean_kana(&raw_kana);
        
        if cleaned_kana.is_empty() {
            return None;
        }
        
        // 提取汉字
        let kanji_selectors = [
            ".head_hyo_1", 
            ".head_hyo_2", 
            ".head_joyo", 
            ".head_kyoiku", 
            ".head_gen"
        ];
        let mut kanji_form = None;
        
        for selector_str in &kanji_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(element) = document.select(&selector).next() {
                    let raw_kanji = element.text().collect::<String>();
                    kanji_form = self.clean_kanji(&raw_kanji);
                    if kanji_form.is_some() {
                        break;
                    }
                }
            }
        }
        
        // 提取发音
        let pronunciation = self.extract_pronunciation(html_content);
        
        // 提取释义
        let meaning = self.extract_meaning(html_content);
        if meaning.is_empty() {
            return None;
        }
        
        // 确定词条类型
        let entry_type = if html_content.contains("item_kanji") {
            "item_kanji"
        } else if html_content.contains("item_ippan") {
            "item_ippan"
        } else if html_content.contains("item_kiso") {
            "item_kiso"
        } else {
            "unknown"
        }.to_string();
        
        Some(DictionaryEntry {
            id: None,
            kana_entry: cleaned_kana,
            kanji_form,
            meaning,
            pronunciation,
            entry_type,
            raw_html: html_content.to_string(),
        })
    }

    /// 从文件中解析所有词条
    pub fn parse_file(&self, file_path: &str) -> Result<Vec<DictionaryEntry>, Box<dyn std::error::Error>> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        
        let mut entries = Vec::new();
        
        println!("🔍 开始解析jpdict.txt文件...");
        let mut line_count = 0;
        
        for line in reader.lines() {
            let line = line?;
            line_count += 1;
            
            if line_count % 10000 == 0 {
                println!("📖 已处理 {} 行，提取到 {} 个词条", line_count, entries.len());
            }
            
            // 检查这一行是否包含完整的词条（以<container开始）
            if line.contains("<container") {
                // 每行都是一个完整的词条，直接解析
                if let Some(entry) = self.parse_entry(&line) {
                    entries.push(entry);
                }
            }
        }
        
        println!("✅ 解析完成！共提取到 {} 个词条", entries.len());
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kana_cleaning() {
        let parser = DictParser::new();
        
        assert_eq!(parser.clean_kana("あい・あお"), "あいあお");
        assert_eq!(parser.clean_kana("アーティスティック-スイミング"), "アーティスティックスイミング");
        assert_eq!(parser.clean_kana("  あい  "), "あい");
    }

    #[test]
    fn test_kanji_cleaning() {
        let parser = DictParser::new();
        
        assert_eq!(parser.clean_kanji("【愛】"), Some("愛".to_string()));
        assert_eq!(parser.clean_kanji("〔英〕"), None);
        assert_eq!(parser.clean_kanji(""), None);
    }
} 