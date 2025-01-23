use std::fmt::Debug;
use rfd::FileDialog;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use dialoguer::{Select, Input};
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, EnumString};
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize)]
struct AppConfig {
    tasks: Vec<String>,
}

impl AppConfig {
    /// 새로운 Task를 추가하고, 바로 파일로 저장합니다.
    pub fn add_task(&mut self, task_name: &str, config_file_path: &str) -> std::io::Result<()> {
        self.tasks.push(task_name.to_string());
        self.save(config_file_path)?;
        Ok(())
    }

    /// 설정 파일에서 config.json을 읽고, 객체로 반환합니다.
    pub fn load(config_file_path: &str) -> Self {
        // config.json이 없으면 디폴트 객체 반환
        if !Path::new(config_file_path).exists() {
            return AppConfig {
                tasks: Vec::new(),
            };
        }

        // 파일 열고 JSON 파싱
        let mut file = File::open(config_file_path)
            .expect("설정 파일을 열 수 없습니다");
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .expect("설정 파일을 읽어올 수 없습니다");

        serde_json::from_str::<AppConfig>(&contents)
            .unwrap_or(AppConfig {
                tasks: Vec::new(),
            })
    }

    /// 현재 객체를 config.json으로 저장합니다.
    pub fn save(&self, config_file_path: &str) -> std::io::Result<()> {
        let json_data = serde_json::to_string_pretty(self)
            .expect("JSON 직렬화에 실패했습니다.");
        let mut file = File::create(config_file_path)?;
        file.write_all(json_data.as_bytes())?;
        Ok(())
    }
}

#[derive(Debug, EnumIter, EnumString)]
enum ExecuteAction {
    Move,
    Copy,
    Delete,
    Unzip,
}

#[derive(Debug, EnumIter, EnumString)]
enum MainMenu {
    Execute,
    Setting,
    Exit,
}

#[derive(Debug, EnumIter, EnumString)]
enum SettingMenu {
    AddTask,
    TaskList,
    Exit,
}

fn main() {
    println!("-------- Easy Patcher By KTS ---------");

    
    let config_file_path = "config.json";
    let mut config = AppConfig::load(config_file_path);
    config.save(config_file_path).unwrap();
    
    load_main_menu();
    
    // 1. 파일 추가 버튼 (CLI 방식으로 처리)
    let file_path = FileDialog::new()
        .pick_file()
        .and_then(|path| Some(path));

    // 2~3. 파일 경로 저장 및 출력
    let file_path = match file_path {
        Some(path) => {
            println!("파일이 선택되었습니다: {}", path.display());
            path
        }
        None => {
            println!("파일 선택이 취소되었습니다.");
            return;
        }
    };

    // 사용자에게 제공할 작업 옵션 제공 (CLI 메뉴)
    let actions: Vec<String> = ExecuteAction::iter()
        .map(|action| format!("{:?}", action))
        .collect();
    
    let selection = Select::new()
        .with_prompt("Choose an action for the file")
        .items(&actions)
        .default(0)
        .interact()
        .unwrap();

    // 사용자 입력에 따른 작업 실행
    match ExecuteAction::from_str(&actions[selection]).unwrap() {
        ExecuteAction::Move => move_file(&file_path),
        ExecuteAction::Copy => copy_file(&file_path),
        ExecuteAction::Delete => delete_file(&file_path),
        ExecuteAction::Unzip => unzip_file(&file_path),
    }
}

fn exit() {
    println!("Exiting Easy Patcher...");
    std::process::exit(0);
}

fn load_setting_menu() {
    let selection = get_selection::<SettingMenu>(Some("choose a menu".to_string()));
    
    match SettingMenu::from_str(&selection).unwrap() {
        SettingMenu::AddTask => add_task(),
        SettingMenu::TaskList => load_task_list(),
        SettingMenu::Exit => load_main_menu(),
    }
}

fn load_main_menu() {
    let selection = get_selection::<MainMenu>(Some("choose a menu".to_string()));

    match MainMenu::from_str(&selection).unwrap() {
        MainMenu::Execute => load_task_list(),
        MainMenu::Setting => load_setting_menu(),
        MainMenu::Exit => exit(),
    }
}

fn add_task() {
    println!("Loading add task menu...");
    todo!()
}

fn load_task_list() {
    println!("Loading task list...");
    todo!()
}

fn get_selection<T: IntoEnumIterator + Debug>(prompt: Option<String>) -> String {
    let items: Vec<String> = T::iter()
        .map(|item| format!("{:?}", item))
        .collect();

    let mut select = Select::new()
        .items(&items)
        .default(0);

    if let Some(prompt) = prompt {
        select = select.with_prompt(prompt);
    }

    let selection = select.interact().unwrap();

    items[selection].clone()
}

/// 파일 이동 함수
fn move_file(file_path: &PathBuf) {
    let new_path: String = Input::new()
        .with_prompt("Enter the destination path to move the file")
        .interact_text()
        .unwrap();
    let target_path = PathBuf::from(new_path);

    if let Err(e) = fs::rename(file_path, target_path) {
        println!("파일 이동 실패: {}", e);
    } else {
        println!("파일이 성공적으로 이동되었습니다.");
    }
}

/// 파일 복사 함수
fn copy_file(file_path: &PathBuf) {
    let new_path: String = Input::new()
        .with_prompt("Enter the destination path to copy the file")
        .interact_text()
        .unwrap();
    let target_path = PathBuf::from(new_path);

    if let Err(e) = fs::copy(file_path, &target_path) {
        println!("파일 복사 실패: {}", e);
    } else {
        println!("파일이 성공적으로 복사되었습니다.");
    }
}

/// 파일 삭제 함수
fn delete_file(file_path: &PathBuf) {
    if let Err(e) = fs::remove_file(file_path) {
        println!("파일 삭제 실패: {}", e);
    } else {
        println!("파일이 성공적으로 삭제되었습니다.");
    }
}

/// 파일 압축 해제 함수 (dummy 구현)
fn unzip_file(file_path: &PathBuf) {
    println!("파일 압축 해제 요청 - 대상 파일: {}", file_path.display());
    // 실제 압축 해제 기능은 추가 라이브러리가 필요합니다.
}
