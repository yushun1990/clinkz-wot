use clinkz_wot_td::{thing::CommonThing, validate::Validate};
use serde_json;
use std::{fs, path::PathBuf};


#[test]
fn test_thing_roundtrip_fidelity() {
    // 使用 CARGO_MANIFEST_DIR 定位子 crate 目录，确保路径鲁棒性
    let mut fixtures_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fixtures_path.push("tests/fixtures");

    let paths = fs::read_dir(&fixtures_path)
        .expect(&format!("Failed to read fixtures at {:?}", fixtures_path));

    for entry in paths {
        let path_buf = entry.unwrap().path();

        // 仅处理 .json 和 .jsonld 文件
        let ext = path_buf.extension().and_then(|s| s.to_str());
        if ext != Some("json") && ext != Some("jsonld") {
            continue;
        }

        // 1. 读取原始数据并解析为通用 Value
        // 这一步是为了后续进行结构对比
        let raw_json = fs::read_to_string(&path_buf).expect("Read failed");
        let mut original_value: serde_json::Value = serde_json::from_str(&raw_json)
            .expect(&format!("Original JSON is invalid: {:?}", path_buf));
        sanitize_json(&mut original_value);

        // 2. 反序列化到你的 Thing 结构体
        let thing: CommonThing = serde_json::from_str(&raw_json)
            .expect(&format!("Failed to deserialize into Thing: {:?}", path_buf));

        // 3. 执行业务逻辑校验
        thing.validate()
            .expect(&format!("Logic validation failed: {:?}", path_buf));

        // 4. 将 Thing 重新序列化回 JSON String
        let serialized_json = serde_json::to_string(&thing)
            .expect(&format!("Failed to serialize: {:?}", path_buf));

        // 5. 将生成的 JSON 解析回 Value 进行等价性对比
        let mut serialized_value: serde_json::Value = serde_json::from_str(&serialized_json).unwrap();
        sanitize_json(&mut serialized_value);

        // 核心优化：比较两个 Value 对象
        // 这样可以忽略字段顺序、空格，并聚焦于数据完整性
        assert_json_eq(&original_value, &serialized_value, &path_buf);
    }
}

fn sanitize_json(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            // 递归清理子项
            map.values_mut().for_each(sanitize_json);
            // 移除空对象、空数组或 null
            map.retain(|_, v| {
                !v.is_null() &&
                !(v.is_object() && v.as_object().unwrap().is_empty()) &&
                !(v.is_array() && v.as_array().unwrap().is_empty())
            });
        }
        serde_json::Value::Array(arr) => {
            arr.iter_mut().for_each(sanitize_json);
        }
        _ => {}
    }
}

/// 辅助函数：深度对比 JSON 内容
fn assert_json_eq(original: &serde_json::Value, serialized: &serde_json::Value, path: &std::path::Path) {
    // 如果原始数据中有 Thing 结构体目前未定义的字段，
    // 它们应该被存储在 _extra_fields (Ext) 中并被重新序列化出来

    if !is_semantic_eq(original, serialized) {
        // 如果不匹配，打印详细的差异（或使用 assert_json_diff crate）
        panic!(
            "Round-trip fidelity check failed for {:?}.\nOriginal: {}\nSerialized: {}",
            path,
            serde_json::to_string_pretty(original).unwrap(),
            serde_json::to_string_pretty(serialized).unwrap()
        );
    }
}

// 辅助函数：尝试以 ISO8601 比较时间
fn try_compare_dates(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    if let (Some(s1), Some(s2)) = (a.as_str(), b.as_str()) {
        // 这里可以引入 chrono 进行真正的解析比较
        // 或者简单地去掉末尾的 'Z' 和多余的 '0' 进行字符串模糊匹配
        return s1.trim_end_matches('Z').trim_end_matches('0') == s2.trim_end_matches('Z').trim_end_matches('0');
    }
    false
}

fn is_semantic_eq(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    use serde_json::Value::*;

    match (a, b) {
        // 1. 处理 WoT 的 OneOrMany 缩写逻辑 (例如 "op": ["read"] 等价于 "op": "read")
        (Array(arr), other) if arr.len() == 1 => is_semantic_eq(&arr[0], other),
        (other, Array(arr)) if arr.len() == 1 => is_semantic_eq(other, &arr[0]),

        // 2. 递归对比 Object，并处理默认值缺失的情况
        (Object(map_a), Object(map_b)) => {
            let all_keys: std::collections::HashSet<_> = map_a.keys().chain(map_b.keys()).collect();
            for key in all_keys {
                let val_a = map_a.get(key).unwrap_or(&Null);
                let val_b = map_b.get(key).unwrap_or(&Null);

                // 如果值不直接相等，检查是否属于“缺失 vs 默认值”的情况
                if val_a != val_b {
                    // 处理日期时间精度：尝试作为日期解析并对比
                    if (key == "created" || key == "modified" || key == "last_changed" || key == "last_updated")
                        && try_compare_dates(val_a, val_b) {
                        continue;
                    }

                    if is_default_value(key, val_a) && val_b.is_null() { continue; }
                    // if is_default_value(key, val_b) && val_a.is_null() { continue; }
                    if !is_semantic_eq(val_a, val_b) { return false; }
                }
            }
            true
        }

        // 3. 数组深度对比
        (Array(arr_a), Array(arr_b)) => {
            if arr_a.len() != arr_b.len() { return false; }
            arr_a.iter().zip(arr_b.iter()).all(|(ia, ib)| is_semantic_eq(ia, ib))
        }

        // 4. 其他基础类型直接对比
        (v1, v2) => v1 == v2,
    }
}

/// 根据 WoT 规范判断是否为默认值
fn is_default_value(key: &str, value: &serde_json::Value) -> bool {
    match key {
        // 布尔类默认值为 false
        "readOnly" | "writeOnly" | "observable" | "safe" | "idempotent" | "success" => {
            value == &serde_json::Value::Bool(false)
        }
        // 也可以在这里扩展其他默认值，例如：
        "contentType" => value == "application/json",
        _ => false,
    }
}
