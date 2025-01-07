use rfd::FileDialog;
use std::fs;
use std::path::PathBuf;
use dialoguer::{Select, Input};

struct File {
    path : String,
    name : String,
}

enum Action {
    Move,
    Copy,
    Delete,
    Unzip,
}

struct Config {
    files : Vec<File>,
    actions : Vec<>
}

fn main() {
    println!("-------- Easy Patcher By KTS ---------");
    
    
    let root_action = vec!["Execute", "Setting", "Exit"];
    
    
    // 1. 파일 추가 버튼 (CLI 방식으로 처리)
    let file_path = FileDialog::new()
        .pick_file() // 파일 선택 대화창 열기
        .and_then(|path| Some(path)); // File Path 처리

    // 2~3. 파일 경로 저장 및 출력
    let file_path = match file_path {
        Some(path) => {
            println!("파일 경로가 선택되었습니다: {}", path.display());
            path
        }
        None => {
            println!("파일 선택이 취소되었습니다.");
            return;
        }
    };

    // 4. 액션 버튼 추가
    println!("\n2. Choose an action for the file!");

    // 사용자에게 제공할 작업 옵션 제공 (CLI 메뉴)
    let actions = vec!["Move", "Copy", "Delete", "Unzip"];
    let selection = Select::new()
        .with_prompt("Choose an action for the file")
        .items(&actions)
        .default(0)
        .interact()
        .unwrap();

    // 사용자 입력에 따른 작업 실행
    match actions[selection] {
        "Move" => move_file(&file_path),
        "Copy" => copy_file(&file_path),
        "Delete" => delete_file(&file_path),
        "Unzip" => unzip_file(&file_path),
        _ => println!("잘못된 작업 선택"),
    }
}

// 5. 각 행동 수행 함수 선언

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

fn check_config_file() -> 