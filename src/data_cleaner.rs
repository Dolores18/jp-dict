use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::collections::HashMap;
use scraper::{Html, Selector};

/// 数据清理器 - 用于清理exported_dict_full.txt文件
pub struct DataCleaner {
    /// 重定向映射表：{重定向源 -> 目标词条}
    pub redirect_map: HashMap<String, String>,
    /// 有效词条统计
    pub valid_entries: usize,
    /// 重定向条目统计  
    pub redirect_entries: usize,
    /// 是否已遇到分界点词条
    boundary_reached: bool,
}

impl DataCleaner {
    pub fn new() -> Self {
        DataCleaner {
            redirect_map: HashMap::new(),
            valid_entries: 0,
            redirect_entries: 0,
            boundary_reached: false,
        }
    }

    /// 清理exported_dict_full.txt文件
    /// 提取所有包含HTML内容的词条，智能解析标题和内容
    pub fn clean_exported_dict(&mut self, input_path: &str, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 开始清理文件: {}", input_path);
        
        let input_file = File::open(input_path)?;
        let reader = BufReader::new(input_file);
        
        let mut output_file = File::create(output_path)?;
        
        for line_result in reader.lines() {
            let line = line_result?;
            
            if line.trim().is_empty() {
                continue;
            }
            
            // 检测重定向行
            if line.starts_with("@@@LINK=") {
                let target = line.strip_prefix("@@@LINK=").unwrap().trim().to_string();
                // 提取前一行可能的标题（这个逻辑我们暂时简化）
                self.redirect_entries += 1;
                continue;
            }
            
            // 检测包含HTML内容的行
            if line.contains("<link rel=\"stylesheet\"") {
                // 这是一个完整的HTML词条
                // 我们需要智能分离标题和HTML内容
                let (title, html_content) = self.extract_title_and_html(&line);
                
                // 输出格式：标题\nHTML内容\n空行
                writeln!(output_file, "{}", title)?;
                writeln!(output_file, "{}", html_content)?;
                writeln!(output_file)?; // 空行分隔
                
                self.valid_entries += 1;
                continue;
            }
        }
        
        println!("✅ 清理完成!");
        println!("📊 统计信息:");
        println!("  - 有效词条: {}", self.valid_entries);
        println!("  - 重定向条目: {}", self.redirect_entries);
        println!("  - 清理后文件: {}", output_path);
        
        Ok(())
    }
    
    /// 从包含HTML的行中提取标题和HTML内容
    fn extract_title_and_html(&self, line: &str) -> (String, String) {
        // 查找HTML开始位置
        if let Some(html_start) = line.find("<link rel=\"stylesheet\"") {
            let html_content = &line[html_start..];
            
            // 从HTML中提取假名和汉字标题
            let title = self.extract_title_from_html(html_content);
            
            (title, html_content.to_string())
        } else {
            // 这种情况不应该发生，但作为备选方案
            ("".to_string(), line.to_string())
        }
    }
    
    /// 从HTML内容中提取标题
    fn extract_title_from_html(&self, html: &str) -> String {
        // 解析HTML
        let document = Html::parse_fragment(html);
        
        // CSS选择器
        let kana_selector = Selector::parse(".headword_kana").unwrap();
        let kanji_selector = Selector::parse(".headword_hyouki").unwrap();
        let ryaku_selector = Selector::parse(".headword_ryaku").unwrap();
        
        let mut kana_reading = String::new();
        let mut kanji_writing = String::new();
        
        // 提取假名读音
        if let Some(kana_element) = document.select(&kana_selector).next() {
            let kana_text = kana_element.text().collect::<String>();
            // 清理假名读音中的特殊符号
            kana_reading = self.clean_kana_text(&kana_text);
        }
        
        // 提取汉字表记
        if let Some(kanji_element) = document.select(&kanji_selector).next() {
            let kanji_text = kanji_element.text().collect::<String>();
            // 清理所有标记符号，只保留汉字、假名和一些基本符号
            kanji_writing = self.clean_kanji_text(&kanji_text);
        }
        
        // 对于英文缩写词条，提取ryaku
        if kana_reading.is_empty() {
            if let Some(ryaku_element) = document.select(&ryaku_selector).next() {
                let ryaku_text = ryaku_element.text().collect::<String>();
                // 对英文缩写也进行清理
                kana_reading = self.clean_kana_text(&ryaku_text);
            }
        }
        
        // 构建标题
        if !kana_reading.is_empty() && !kanji_writing.is_empty() {
            format!("{}【{}】", kana_reading, kanji_writing)
        } else if !kana_reading.is_empty() {
            kana_reading
        } else {
            // 如果找不到标题，尝试提取data-id作为标识
            if let Some(container) = document.select(&Selector::parse("container").unwrap()).next() {
                if let Some(data_id) = container.value().attr("data-id") {
                    format!("entry_{}", data_id)
                } else {
                    "unknown_entry".to_string()
                }
            } else {
                "unknown_entry".to_string()
            }
        }
    }
    
    /// 清理汉字文本，只保留汉字、假名和基本符号
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
                // 过滤掉标记符号：【】◇△▽▲▼○●◯□■▢▣◆◇※等
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
    
    /// 清理HTML标签，只保留文本内容（保留作为备用方法）
    fn clean_html_tags(&self, html: &str) -> String {
        // 使用正则表达式移除所有HTML标签
        let re = regex::Regex::new(r"<[^>]*>").unwrap();
        let cleaned = re.replace_all(html, "");
        
        // 清理多余的空白
        cleaned.trim().to_string()
    }

    /// 清理假名读音，去除特殊符号
    fn clean_kana_text(&self, text: &str) -> String {
        let mut result = String::new();
        for ch in text.chars() {
            match ch {
                // 保留平假名
                '\u{3040}'..='\u{309f}' => result.push(ch),
                // 保留片假名
                '\u{30a0}'..='\u{30ff}' => result.push(ch),
                // 保留一些基本符号
                '・' | '‧' | '·' | '-' | 'ー' => result.push(ch),
                // 过滤掉标记符号：【】◇△▽▲▼○●◯□■▢▣◆◇※等
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
    
    /// 检查是否是汉字重定向的开始
    fn is_kanji_redirect_start(&self, line: &str) -> bool {
        let line = line.trim();
        // 检查是否是单独的汉字行（1-3个字符，主要由汉字组成）
        line.len() >= 1 && line.len() <= 3 && self.is_likely_kanji_only(line)
    }
    
    /// 检测是否到达分界点（汉字重定向区域开始）
    fn is_boundary_reached(&self, current_line: &str) -> Result<bool, Box<dyn std::error::Error>> {
        // 精确的分界点检测：使用data-id="3011400"作为分界点
        // 这是ヴォ词条，之后紧接着就是汉字重定向区域
        
        if current_line.contains("data-id=") {
            if let Some(id_str) = self.extract_data_id(current_line) {
                if id_str == "3011400" {
                    println!("🎯 检测到精确分界点: data-id={} (ヴォ词条)", id_str);
                    println!("📍 此词条之后开始汉字重定向区域，将在处理完此词条后停止");
                    return Ok(false); // 先处理完这个词条
                }
            }
        }
        
        // 检测汉字行：如果已经处理了data-id=3011400的词条，遇到汉字行就停止
        let line = current_line.trim();
        if line.len() >= 1 && line.len() <= 3 && self.is_likely_kanji_only(line) {
            if self.valid_entries > 0 {
                println!("🔍 确认分界点: 汉字行 '{}' (已处理{}个词条)", line, self.valid_entries);
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    /// 从HTML行中提取data-id值
    fn extract_data_id(&self, line: &str) -> Option<String> {
        if let Some(start) = line.find("data-id=\"") {
            let start = start + 9; // "data-id=\"".len()
            if let Some(end) = line[start..].find("\"") {
                return Some(line[start..start + end].to_string());
            }
        }
        None
    }
    
    /// 判断是否是纯汉字行
    fn is_likely_kanji_only(&self, line: &str) -> bool {
        let line = line.trim();
        if line.is_empty() {
            return false;
        }
        
        // 检查是否主要由汉字组成
        let kanji_count = line.chars().filter(|c| {
            *c >= '\u{4e00}' && *c <= '\u{9fff}' // CJK统一汉字
        }).count();
        
        let total_chars = line.chars().count();
        
        // 如果80%以上是汉字，认为是汉字行
        kanji_count > 0 && kanji_count as f32 / total_chars as f32 > 0.8
    }
    
    /// 分析文件结构，不进行清理，只统计
    pub fn analyze_file_structure(&mut self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 分析文件结构: {}", file_path);
        
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        
        let mut total_lines = 0;
        let mut html_entries = 0;
        let mut redirect_lines = 0;
        let mut boundary_found = false;
        
        for line_result in reader.lines() {
            let line = line_result?;
            total_lines += 1;
            
            if line.starts_with("@@@LINK=") {
                redirect_lines += 1;
            } else if line.contains("<link rel=\"stylesheet\"") {
                html_entries += 1;
            }
            
            // 检测分界点附近
            if !boundary_found && total_lines > 300000 && self.is_likely_kanji_only(&line) {
                println!("🔍 疑似分界点位置: 第{}行 - {}", total_lines, line);
                boundary_found = true;
            }
        }
        
        println!("📊 文件结构分析:");
        println!("  - 总行数: {}", total_lines);
        println!("  - HTML词条: {}", html_entries);
        println!("  - 重定向行: {}", redirect_lines);
        println!("  - 预计有效词条: {}", html_entries);
        
        Ok(())
    }
    
    /// 获取统计信息
    pub fn get_stats(&self) -> (usize, usize, usize) {
        (self.valid_entries, self.redirect_entries, self.redirect_map.len())
    }
} 