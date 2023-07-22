extern crate lalrpop;

use std::fs::remove_dir_all;

fn main() {
    //lalrpop::process_root().unwrap(); // build to cargo tmp

    // build to src
    // 最多尝试构建三次
    let mut res = Ok(());
    for _ in 0..3 {
        res = lalrpop::Configuration::new()
            .generate_in_source_tree()
            .always_use_colors()
            .process();
        if res.is_ok() {
            break;
        }
        // 可能是由于增量构建带来的位置转移导致构建失败
        // 删除增量并尝试下一次构建
        remove_incremental();
    }
    res.unwrap();
}

/// 删除增量文件
#[allow(unused)]
fn remove_incremental() {
    let incremental_path = [
        env!("CARGO_MANIFEST_DIR"),
        "/target/",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        "/incremental"
    ].concat();
    remove_dir_all(incremental_path).unwrap();
}
