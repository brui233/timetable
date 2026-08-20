use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScheduleData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl Default for ScheduleData {
    fn default() -> Self {
        ScheduleData {
            headers: vec![
                "节次".into(),
                "星期一".into(),
                "星期二".into(),
                "星期三".into(),
                "星期四".into(),
                "星期五".into(),
                "星期六".into(),
                "星期日".into(),
            ],
            rows: vec![
                vec!["1-2节".into(), "".into(), "".into(), "".into(), "".into(), "".into(), "".into(), "".into()],
                vec!["3-4节".into(), "".into(), "".into(), "".into(), "".into(), "".into(), "".into(), "".into()],
                vec!["5-6节".into(), "".into(), "".into(), "".into(), "".into(), "".into(), "".into(), "".into()],
            ],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Config {
    pub start_date: Option<String>,
}

fn data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法获取应用数据目录: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("创建数据目录失败: {e}"))?;
    Ok(dir)
}

#[tauri::command]
fn load_schedule(app: tauri::AppHandle) -> Result<ScheduleData, String> {
    let path = data_dir(&app)?.join("schedule.json");
    if !path.exists() {
        return Ok(ScheduleData::default());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("读取课表失败: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("解析课表失败: {e}"))
}

#[tauri::command]
fn save_schedule(app: tauri::AppHandle, data: ScheduleData) -> Result<(), String> {
    let path = data_dir(&app)?.join("schedule.json");
    let content = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| format!("保存课表失败: {e}"))
}

#[tauri::command]
fn load_config(app: tauri::AppHandle) -> Result<Config, String> {
    let path = data_dir(&app)?.join("config.json");
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("读取配置失败: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("解析配置失败: {e}"))
}

#[tauri::command]
fn save_config(app: tauri::AppHandle, config: Config) -> Result<(), String> {
    let path = data_dir(&app)?.join("config.json");
    let content = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| format!("保存配置失败: {e}"))
}

/// 从 Excel (xlsx/xls) 或 CSV 文件导入课表
#[tauri::command]
fn import_file(path: String) -> Result<ScheduleData, String> {
    let lower = path.to_lowercase();
    if lower.ends_with(".csv") {
        import_csv(&path)
    } else if lower.ends_with(".xlsx") || lower.ends_with(".xls") {
        import_excel(&path)
    } else {
        Err("不支持的文件格式，请选择 .xlsx / .xls / .csv 文件".into())
    }
}

fn import_excel(path: &str) -> Result<ScheduleData, String> {
    use calamine::{open_workbook_auto, Reader};
    let mut workbook = open_workbook_auto(path).map_err(|e| format!("打开 Excel 失败: {e}"))?;
    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or("Excel 文件中没有工作表")?;
    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|e| format!("读取工作表失败: {e}"))?;

    let mut rows_iter = range.rows();
    let headers: Vec<String> = match rows_iter.next() {
        Some(r) => r.iter().map(|c| c.to_string().trim().to_string()).collect(),
        None => return Err("Excel 文件为空".into()),
    };
    let rows: Vec<Vec<String>> = rows_iter
        .map(|r| r.iter().map(|c| c.to_string().trim().to_string()).collect())
        .collect();

    Ok(ScheduleData { headers, rows })
}

fn import_csv(path: &str) -> Result<ScheduleData, String> {
    let bytes = fs::read(path).map_err(|e| format!("读取文件失败: {e}"))?;
    // 优先按 UTF-8 解码，失败则按 GBK 解码，避免中文乱码
    let text = match String::from_utf8(bytes.clone()) {
        Ok(s) => s,
        Err(_) => {
            let (decoded, _, had_errors) = encoding_rs::GBK.decode(&bytes);
            if had_errors {
                return Err("无法识别文件编码（既不是 UTF-8 也不是 GBK）".into());
            }
            decoded.into_owned()
        }
    };

    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(text.as_bytes());

    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| format!("解析表头失败: {e}"))?
        .iter()
        .map(|s| s.trim().to_string())
        .collect();

    let mut rows: Vec<Vec<String>> = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| format!("解析行失败: {e}"))?;
        rows.push(record.iter().map(|s| s.trim().to_string()).collect());
    }

    Ok(ScheduleData { headers, rows })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            load_schedule,
            save_schedule,
            load_config,
            save_config,
            import_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
