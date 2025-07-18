mod database;
mod parser;
mod obunsha_dict;  // 新增：旺文社国語辞典模块
mod data_cleaner;  // 新增：数据清理模块
mod web_server;  
use database::{Database, DictionaryEntry};
use parser::DictParser;
use obunsha_dict::ObunshaDictDatabase;  // 移除未使用的ObunshaDictEntry
use data_cleaner::DataCleaner;  // 新增：数据清理器导入
use std::env;
use web_server::start_server;  // 修正：使用正确的函数名
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
        "init-obunsha" => {  // 新增：初始化旺文社国语辞典表
            init_obunsha_table()
        }
        "clean-data" => {  // 新增：清理导出数据
            clean_exported_data()
        }
        "analyze-data" => {  // 新增：分析数据结构
            analyze_exported_data()
        }
        "import-obunsha" => {  // 新增：导入旺文社数据到数据库
            import_obunsha_data()
        }
        "server" => {  // 新增：启动Web服务器
            start_web_server()
        }
        _ => {
            println!("使用方法:");
            println!("  extract      - 提取词典数据");
            println!("  test-agaku   - 测试あがく词条解析");
            println!("  init-obunsha - 初始化旺文社国语辞典表");
            println!("  clean-data   - 清理exported_dict_full.txt");
            println!("  analyze-data - 分析exported_dict_full.txt结构");
            println!("  import-obunsha - 导入清理后的数据到旺文社数据库");
            println!("  server       - 启动Web API服务器");
            Ok(())
        }
    }
}

/// 初始化旺文社国語辞典表
fn init_obunsha_table() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 初始化旺文社国語辞典数据库表...");
    
    let db = ObunshaDictDatabase::new("obunsha_dict.db")?;
    db.initialize()?;
    
    let (count, unique) = db.get_stats()?;
    println!("📊 当前表统计: {} 条词条, {} 个唯一标题", count, unique);
    
    println!("✅ 表结构初始化完成!");
    Ok(())
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

/// 清理导出的字典数据
fn clean_exported_data() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧹 清理exported_dict_full.txt数据...");
    
    let mut cleaner = DataCleaner::new();
    cleaner.clean_exported_dict("exported_dict_full.txt", "exported_dict_cleaned.txt")?;
    
    let (valid, redirects, mappings) = cleaner.get_stats();
    println!("📈 清理结果:");
    println!("  - 有效词条: {}", valid);
    println!("  - 重定向记录: {}", redirects);
    println!("  - 映射关系: {}", mappings);
    
    Ok(())
}

/// 分析导出数据的结构
fn analyze_exported_data() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 分析exported_dict_full.txt结构...");
    
    let mut cleaner = DataCleaner::new();
    cleaner.analyze_file_structure("exported_dict_full.txt")?;
    
    Ok(())
}

/// 导入清理后的数据到旺文社数据库
fn import_obunsha_data() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 导入清理后的数据到旺文社数据库...");
    
    let cleaned_data_path = "exported_dict_cleaned.txt";
    let db = ObunshaDictDatabase::new("obunsha_dict.db")?;
    
    // 确保表已经初始化
    db.initialize()?;
    
    println!("📖 开始从清理数据导入词条: {}", cleaned_data_path);
    let imported_count = db.import_from_cleaned_data(cleaned_data_path)?;
    
    let (total_count, unique_headwords) = db.get_stats()?;
    println!("🎉 数据导入完成！");
    println!("📊 本次导入: {} 条词条", imported_count);
    println!("📊 数据库总计: {} 条词条, {} 个唯一标题", total_count, unique_headwords);
    
    Ok(())
}

/// 启动Web服务器
fn start_web_server() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌐 启动旺文社词典Web服务器...");
    
    // 检查数据库文件是否存在
    let db_path = "obunsha_dict.db";
    if !std::path::Path::new(db_path).exists() {
        println!("❌ 错误：数据库文件 {} 不存在", db_path);
        println!("💡 请先运行 'cargo run import-obunsha' 创建数据库");
        return Ok(());
    }
    
    // 验证数据库连接
    match ObunshaDictDatabase::new(db_path) {
        Ok(db) => {
            let (count, _) = db.get_stats().unwrap_or((0, 0));
            if count == 0 {
                println!("⚠️  警告：数据库为空，请先导入数据");
                return Ok(());
            }
            println!("📚 数据库连接成功，共有 {} 个词条", count);
        }
        Err(e) => {
            println!("❌ 数据库连接失败: {}", e);
            return Ok(());
        }
    }
    
    // 使用tokio运行时启动服务器
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        if let Err(e) = start_server(db_path, 3000).await {
            println!("❌ 服务器启动失败: {}", e);
        }
    });
    
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