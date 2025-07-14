use std::env;
use std::fs;
use mdict_parser::parser;

fn main() {
    println!("MDX词典解析器");
    
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        println!("用法: cargo run --bin mdx_parser <mdx文件路径> [--verbose]");
        println!("示例: cargo run --bin mdx_parser data/dictionary.mdx");
        println!("详细模式: cargo run --bin mdx_parser data/dictionary.mdx --verbose");
        return;
    }
    
    let mdx_file_path = &args[1];
    let verbose = args.len() > 2 && args[2] == "--verbose";
    
    println!("正在解析MDX文件: {}", mdx_file_path);
    
    // 读取MDX文件
    match fs::read(mdx_file_path) {
        Ok(data) => {
            println!("文件大小: {:.2} MB", data.len() as f64 / 1024.0 / 1024.0);
            
            // 使用mdict-parser解析
            let dict = parser::parse(&data);
            println!("✅ MDX文件解析成功!");
            
            // 获取所有词条的键
            let keys: Vec<_> = dict.keys().collect();
            println!("📊 词条总数: {}", keys.len());
            
            if verbose {
                // 详细模式：显示前10个词条
                println!("\n📝 前10个词条:");
                for (i, key) in keys.iter().take(10).enumerate() {
                    println!("{}. {:?}", i + 1, key);
                }
                
                // 显示前3个词条的详细信息
                println!("\n📖 前3个词条的详细信息:");
                for (i, item) in dict.items().enumerate() {
                    if i >= 3 { break; }
                    println!("{}. {:?}", i + 1, item);
                }
            } else {
                // 简洁模式：显示前5个词条的完整内容
                println!("\n📝 前5个词条的完整内容:");
                for (i, record) in dict.items().enumerate() {
                    if i >= 5 { break; }
                    println!("\n{}. 词条: {:?}", i + 1, record.key);
                    println!("   定义: {:?}", record.definition);
                }
                println!("\n💡 使用 --verbose 参数查看更多详细信息");
            }
        },
        Err(e) => {
            eprintln!("❌ 读取文件失败: {}", e);
            eprintln!("请确认文件路径是否正确: {}", mdx_file_path);
        }
    }
} 