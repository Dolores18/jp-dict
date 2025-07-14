mod database;
mod parser;

use database::{Database, DictionaryEntry};
use parser::DictParser;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("表現読解国語辞典 - 日语词典数据提取工具");
    
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("test");
    
    match mode {
        "extract" => {
            extract_dictionary_data()
        }
        "test-agaku" => {
            test_agaku_parsing()
        }
        _ => {
            test_database_structure()
        }
    }
}

fn test_agaku_parsing() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 测试あがく词条解析...");
    
    let parser = DictParser::new();
    let test_html = r#"<link rel="stylesheet" href="style.css"><container data-id="236" data-type="2"><div id="id_00000236" class="item item_ippan"><div class="head"><span class="head_kana">あが・く</span><span class="head_hyo_2"><span class="ka_hyo_2">〖</span><span class="mj_sup">◇</span>足<span class="mj_sup">△</span>搔く<span class="ka_hyo_2">〗</span></span></div><div class="mean_normal"><span class="pos"><span class="ka_pos_s">（</span>自五<span class="ka_pos_e">）</span></span><span class="ka_inflec">｛</span><span class="inflec">カ<span class="mj_inflec">（</span>コ<span class="mj_inflec">）</span>・キ<span class="mj_inflec">（</span>イ<span class="mj_inflec">）</span>・<br>ク・ク・ケ・ケ</span><span class="ka_inflec">｝</span></div><div class="mean_lv_2 mean_no_1">❶手足を動かしてもがく。じたばたする。<span class="ex_text">組み敷<span class="mlg mlg_1">し</span>かれて━</span></div><div class="mean_lv_2 mean_no_2">❷悪い状況<span class="mlg mlg_6">じようきよう</span>からぬけ出そうとして、いろいろむだな試みをする。<span class="ex_text">今さら━・いてもむだだ</span></div></div></contaienr></html>"#;
    
    if let Some(entry) = parser.parse_entry(test_html) {
        println!("✅ 解析成功！");
        println!("假名: {}", entry.kana_entry);
        println!("汉字: {:?}", entry.kanji_form);
        println!("发音: {:?}", entry.pronunciation);
        println!("释义: {}", entry.meaning);
        println!("类型: {}", entry.entry_type);
    } else {
        println!("❌ 解析失败！");
    }
    
    Ok(())
}

/// 从jpdict.txt提取数据到数据库
fn extract_dictionary_data() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 开始从jpdict.txt提取词典数据...");
    
    // 创建数据库连接
    let db = Database::new("dictionary.db")?;
    db.initialize()?;
    println!("✅ 数据库连接建立成功");
    
    // 清空现有数据（如果需要重新导入）
    println!("⚠️  清空现有数据...");
    db.clear_all_entries()?;
    println!("✅ 数据库已清空");
    
    // 创建解析器
    let parser = DictParser::new();
    
    // 解析jpdict.txt文件
    let jpdict_path = "data/jpdict.txt";
    println!("📖 开始解析文件: {}", jpdict_path);
    
    let entries = parser.parse_file(jpdict_path)?;
    println!("📊 解析完成，共提取到 {} 个词条", entries.len());
    
    // 分批插入数据库（每1000条一批）
    let batch_size = 1000;
    let total_batches = (entries.len() + batch_size - 1) / batch_size;
    
    println!("💾 开始插入数据库，共 {} 批次...", total_batches);
    
    for (batch_idx, chunk) in entries.chunks(batch_size).enumerate() {
        println!("📥 正在插入第 {}/{} 批次（{} 条）...", 
                batch_idx + 1, total_batches, chunk.len());
        
        db.insert_entries_batch(chunk)?;
    }
    
    // 显示最终统计
    let final_count = db.get_entry_count()?;
    println!("🎉 数据导入完成！");
    println!("📊 数据库中共有 {} 个词条", final_count);
    
    // 显示一些示例数据
    println!("\n📝 示例数据:");
    let sample_entries = db.find_by_kana("あい")?;
    for (i, entry) in sample_entries.iter().take(3).enumerate() {
        println!("  {}. 假名: {} | 汉字: {:?} | 发音: {:?} | 类型: {}", 
                i + 1, entry.kana_entry, entry.kanji_form, 
                entry.pronunciation, entry.entry_type);
    }
    
    Ok(())
}

/// 测试数据库结构
fn test_database_structure() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 测试数据库结构...");
    
    // 创建数据库连接
    let db = Database::new("dictionary.db")?;
    
    // 初始化数据库表结构
    db.initialize()?;
    println!("✅ 数据库表 'dictionary_entries' 创建成功");
    
    // 插入一个测试词条来验证结构
    let test_entry = DictionaryEntry {
        id: None,
        kana_entry: "あい".to_string(),
        kanji_form: Some("愛".to_string()),
        meaning: "❶かわいがりいつくしむ気持ち。❷こいしたう気持ち。❸たいせつに思う気持ち。".to_string(),
        pronunciation: Some("アイ".to_string()),
        entry_type: "item_kiso".to_string(),
        raw_html: r#"<div class="item item_kiso"><div class="head"><span class="head_kana">あい</span><span class="head_hyo_1">【愛】</span></div></div>"#.to_string(),
    };
    
    let entry_id = db.insert_entry(&test_entry)?;
    println!("✅ 测试词条插入成功，ID: {}", entry_id);
    
    // 检查词条总数
    let count = db.get_entry_count()?;
    println!("📊 当前词典中共有 {} 个词条", count);
    
    // 根据假名查询测试
    let found_entries = db.find_by_kana("あい")?;
    println!("🔍 找到 {} 个匹配 'あい' 的词条", found_entries.len());
    
    for entry in found_entries {
        println!("  - 假名: {}", entry.kana_entry);
        if let Some(kanji) = &entry.kanji_form {
            println!("    汉字: {}", kanji);
        }
        if let Some(pronunciation) = &entry.pronunciation {
            println!("    发音: {}", pronunciation);
        }
        println!("    释义: {}", entry.meaning);
        println!("    类型: {}", entry.entry_type);
        println!();
    }
    
    println!("🎉 数据库结构验证完成！");
    println!("💡 运行 'cargo run extract' 开始提取jpdict.txt数据");
    
    Ok(())
}
