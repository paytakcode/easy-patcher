use dialoguer::{Input, Select};
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, EnumString};
use zip::read::ZipArchive;

const CONFIG_FILE_PATH: &str = "config.json";

#[derive(Debug, Serialize, Deserialize)]
struct Task {
    name: String,
    target_files: Vec<TargetFile>,
    actions: Vec<Action>,
    projects: Vec<Project>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Project {
    path: String,
    vcs_type: VcsType,
}

#[derive(Debug, Serialize, Deserialize)]
enum VcsType {
    Git,
    Svn,
    Unknown,
}

impl Task {
    pub fn new(name: String) -> Self {
        Task {
            name,
            target_files: Vec::new(),
            actions: Vec::new(),
            projects: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct TargetFile {
    name: String,
    path: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Action {
    target_file: TargetFile,
    destination: Option<String>,
    command: Command,
}

impl Action {
    pub fn delete_action(target_file: &TargetFile) -> Self {
        Action {
            target_file: TargetFile {
                name: target_file.name.clone(),
                path: target_file.path.clone(),
            },
            destination: None,
            command: Command::Delete,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct FileOperation {
    sequence: u8,
    target_file: TargetFile,
    command: Command,
}

#[derive(Debug, Serialize, Deserialize)]
struct AppConfig {
    tasks: Vec<Task>,
}

impl AppConfig {
    pub fn add_task(&mut self, task_name: &str) -> std::io::Result<()> {
        let mut config = AppConfig::load();
        let existing_task
            = config.tasks.iter().find(|task| task.name == task_name);
        if existing_task
            .is_some() {
            println!("there is already a task with the same name.");
            add_task(&mut config);
        }
        self.tasks.push(Task::new(task_name.to_string()));
        self.save()?;
        Ok(())
    }

    pub fn load() -> Self {
        // config.json이 없으면 디폴트 객체 반환
        if !Path::new(CONFIG_FILE_PATH).exists() {
            return AppConfig {
                tasks: Vec::new(),
            };
        }

        let mut file = File::open(CONFIG_FILE_PATH)
            .expect("설정 파일을 열 수 없습니다");
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .expect("설정 파일을 읽어올 수 없습니다");

        serde_json::from_str::<AppConfig>(&contents)
            .unwrap_or(AppConfig {
                tasks: Vec::new(),
            })
    }
    
    pub fn get_task(&self, task_name: &str) -> Option<&Task> {
        self.tasks.iter().find(|task| task.name == task_name)
    }
    
    pub fn get_task_mut(&mut self, task_name: &str) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|task| task.name == task_name)
    }

    pub fn save(&self) -> std::io::Result<()> {
        let json_data = serde_json::to_string_pretty(self)
            .expect("JSON 직렬화에 실패했습니다.");
        let mut file = File::create(CONFIG_FILE_PATH)?;
        file.write_all(json_data.as_bytes())?;
        Ok(())
    }
}

#[derive(Debug, EnumIter, EnumString, Serialize, Deserialize)]
enum Command {
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
    NewTask,
    TaskList,
    Back,
}

#[derive(Debug, EnumIter, EnumString)]
enum TaskMenu {
    NewAction,
    AddTargetFile,
    AddProject,
    ActionList,
    ProjectList,
    DeleteTask,
    Back,
}

fn main() {
    let mut config = AppConfig::load();

    loop {
        // 메인 메뉴 선택
        let selection = get_selection_from_enum::<MainMenu>(Some("---------- Easy Patcher ----------\nchoose a menu"));
        match MainMenu::from_str(&selection).unwrap() {
            MainMenu::Execute => {
                run_execute_menu(&mut config);
            },
            MainMenu::Setting => {
                run_setting_menu(&mut config);
            },
            MainMenu::Exit => {
                println!("Exiting Easy Patcher...");
                break;
            },
        }
    }
}

/// 실행(Execute) 메뉴 흐름
fn run_execute_menu(config: &mut AppConfig) {
    let mut task_names: Vec<String> = config.tasks
        .iter()
        .map(|task| task.name.clone())
        .collect();
    task_names.push("Back".to_string());

    let selected_task = get_selection_from_str(task_names, Some("실행할 Task를 선택하세요"));
    if selected_task == "Back" {
        return;
    }
    execute_action_list(config, &selected_task);
}

/// 설정(Setting) 메뉴 흐름
fn run_setting_menu(config: &mut AppConfig) {
    loop {
        let selection = get_selection_from_enum::<SettingMenu>(Some("---------- Setting Menu ----------"));
        match SettingMenu::from_str(&selection).unwrap() {
            SettingMenu::NewTask => {
                add_task(config);
            },
            SettingMenu::TaskList => {
                show_task_list(config);
            },
            SettingMenu::Back => {
                // 설정 메뉴 탈출
                break;
            },
        }
    }
}

fn main_menu() {
    let selection = get_selection_from_enum::<MainMenu>(Some("---------- Easy Patcher ----------\nchoose a menu"));

    let mut config = AppConfig::load();
    match MainMenu::from_str(&selection).unwrap() {        
        MainMenu::Execute => {
            // 실행할 Task를 선택하고 액션 실행
            let mut task_names = config.tasks
                .iter()
                .map(|task| task.name.clone())
                .collect::<Vec<String>>();
            task_names.push("Back".to_string());
    
            let selected_task = get_selection_from_str(task_names, Some("실행할 Task를 선택하세요"));
            if selected_task == "Back" {
                main_menu();
            } else {
                execute_action_list(&config , &selected_task);
                // 실행 후 필요하다면 다시 메뉴로 돌아가기
                main_menu();
            }
        },
        MainMenu::Setting => setting_menu(&mut config),
        MainMenu::Exit => exit(),
    }
}
/// Task 추가 함수
fn add_task(config: &mut AppConfig) {
    let task_name: String = Input::new()
        .with_prompt("Enter new task name:")
        .interact_text()
        .unwrap();

    match config.add_task(&task_name) {
        Ok(_) => {
            if let Err(e) = config.save() {
                println!("save error: {}", e);
            }
        },
        Err(msg) => println!("{}", msg),
    }
}

/// Task 목록을 보여주고, 선택된 Task에 대해 TaskMenu 실행
fn show_task_list(config: &mut AppConfig) {
    let mut task_names = config.tasks
        .iter()
        .map(|task| task.name.clone())
        .collect::<Vec<String>>();
    task_names.push("Back".to_string());
    let selection = get_selection_from_str(task_names, Some("choose a task"));
    if selection == "Back" {
        return;
    }

    loop {
        let menu_sel = get_selection_from_enum::<TaskMenu>(Some("choose menu for task"));
        match TaskMenu::from_str(&menu_sel).unwrap() {
            TaskMenu::NewAction => add_action(config, &selection),
            TaskMenu::AddTargetFile => add_target_file(config, &selection),
            TaskMenu::AddProject => {
                add_project(config, &selection);
            },
            TaskMenu::ActionList => action_list(config, &selection),
            TaskMenu::ProjectList => show_project_history_menu(config, &selection),
            TaskMenu::DeleteTask => {
                delete_task(config, &selection);
                break; // 삭제 후 목록으로 돌아가기
            },
            TaskMenu::Back => {
                break; // Task 메뉴 탈출 → TaskList 메뉴로 복귀
            },
        }
    }
}

fn add_project(config: &mut AppConfig, task_name: &str) {
    if let Some(task) = config.get_task_mut(task_name) {
        println!("프로젝트 폴더를 선택하세요:");
        // 폴더 선택
        if let Some(folder_path) = FileDialog::new().pick_folder() {
            let folder_str = folder_path.display().to_string();
            let git_path = folder_path.join(".git");
            let svn_path = folder_path.join(".svn");

            // VCS 종류 판별
            let vcs_type = if git_path.exists() {
                VcsType::Git
            } else if svn_path.exists() {
                VcsType::Svn
            } else {
                VcsType::Unknown
            };

            // Project 객체 생성 후 Task에 추가
            let new_project = Project {
                path: folder_str,
                vcs_type,
            };
            task.projects.push(new_project);

            if let Err(e) = config.save() {
                println!("프로젝트 정보 저장 중 오류: {}", e);
            } else {
                println!("프로젝트가 성공적으로 추가되었습니다!");
            }
        } else {
            println!("프로젝트 폴더 선택이 취소되었습니다.");
        }
    }
}

/// 특정 Task 삭제
fn delete_task(config: &mut AppConfig, task_name: &str) {
    config.tasks.retain(|task| task.name != task_name);
    if let Err(e) = config.save() {
        println!("delete_task save error: {}", e);
    }
}
/// 특정 Task의 Action을 순차적으로 실행하는 함수
fn execute_action_list(config: &AppConfig, task_name: &str) {
    if let Some(task) = config.get_task(task_name) {
        for action in &task.actions {
            match action.command {
                Command::Move => {
                    println!("파일 이동 실행 중");
                    move_file(&action.target_file.path, action.destination.as_ref().unwrap());
                },
                Command::Copy => {
                    println!("파일 복사 실행 중: {}", action.target_file.path);
                    copy_file(&PathBuf::from(&action.target_file.path));
                },
                Command::Delete => {
                    println!("파일 삭제 실행 중: {}", action.target_file.path);
                    delete_file(&action.target_file.path);
                },
                Command::Unzip => {
                    println!("파일 압축 해제 실행 중: {} to {}", &action.target_file.path, &action.destination.as_ref().unwrap());
                    unzip_file(&action.target_file.path, action.destination.as_ref().unwrap()).unwrap();
                },
            }
        }
    } else {
        println!("해당 Task를 찾을 수 없습니다.");
    }
}

/// Action 목록 출력
fn action_list(config: &mut AppConfig, task_name: &str) {
    if let Some(task) = config.get_task(task_name) {
        for (index, action) in task.actions.iter().enumerate() {
            println!("{}: {:?}", index, action);
        }
    }
}
/// 새 Action 추가
fn add_action(config: &mut AppConfig, task_name: &str) {
    command_list(config, task_name);
}

fn setting_menu(config: &mut AppConfig) {
    let selection = get_selection_from_enum::<SettingMenu>(None);

    match SettingMenu::from_str(&selection).unwrap() {
        SettingMenu::NewTask => add_task(config),
        SettingMenu::TaskList => task_list(),
        SettingMenu::Back => main_menu(),
    }
}

fn task_list() {
    let mut config = AppConfig::load();
    let mut task_names = config.tasks.iter().map(|task| task.name.clone()).collect::<Vec<String>>();
    task_names.push("Back".to_string());
    let selection = get_selection_from_str(task_names, Some("choose a task"));
    if selection == "Back" {
        setting_menu(&mut config);
    } else { 
        task_menu(&selection);
    }
}

/// Enum 선택 시 사용
fn get_selection_from_enum<T: IntoEnumIterator + Debug>(prompt: Option<&str>) -> String {
    let items: Vec<String> = T::iter().map(|item| format!("{:?}", item)).collect();
    let mut select = Select::new().items(&items).default(0);

    if let Some(text) = prompt {
        select = select.with_prompt(text);
    }
    let selection = select.interact().unwrap();
    items[selection].clone()
}

/// 일반 String 목록 선택 시 사용
fn get_selection_from_str(items: Vec<String>, prompt: Option<&str>) -> String {
    let mut select = Select::new().items(&items).default(0);

    if let Some(text) = prompt {
        select = select.with_prompt(text);
    }
    let selection = select.interact().unwrap();
    items[selection].clone()
}

/// 파일 이동 함수
fn move_file(source: &str, destination: &str) {
    if let Err(e) = fs::rename(source, destination) {
        println!("파일 이동 실패: {}", e);
    } else {
        println!("파일이 성공적으로 이동되었습니다.");
    }
}

fn copy_file(source: &PathBuf) {
    let new_path: String = Input::new()
        .with_prompt("Enter the destination path to copy the file")
        .interact_text()
        .unwrap();
    let target_path = PathBuf::from(new_path);

    if let Err(e) = fs::copy(source, &target_path) {
        println!("파일 복사 실패: {}", e);
    } else {
        println!("파일이 성공적으로 복사되었습니다.");
    }
}

/// 파일 삭제 함수
fn delete_file(target_file: &String) {
    let path = Path::new(target_file);
    if let Err(e) = fs::remove_file(path) {
        println!("파일 삭제 실패: {}", e);
    } else {
        println!("파일이 성공적으로 삭제되었습니다.");
    }
}

/// 파일 압축 해제 함수 (dummy 구현)
fn unzip_file(zip_path: &str, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file_in_zip = archive.by_index(i)?;
        let out_path = Path::new(output_dir);

        if file_in_zip.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            let mut out_file = File::create(&out_path)?;
            std::io::copy(&mut file_in_zip, &mut out_file)?;
        }
    }
    Ok(())
}


fn task_menu(task_name: &str) {
    let mut config = AppConfig::load();
    let selection = get_selection_from_enum::<TaskMenu>(Some("choose menu for task"));

    match TaskMenu::from_str(&selection).unwrap() {
        TaskMenu::NewAction => add_action(&mut config, task_name),
        TaskMenu::AddTargetFile => add_target_file(&mut config, task_name),
        TaskMenu::AddProject => add_project(&mut config, task_name),
        TaskMenu::ActionList => action_list(&mut config, task_name),
        TaskMenu::ProjectList => show_project_history_menu(&mut config, task_name),
        TaskMenu::DeleteTask => delete_task(&mut config, task_name),
        TaskMenu::Back => task_list(),
    }
    
}

/// 명령어 목록 중에서 하나를 선택한 후 해당 Action 추가
fn command_list(config: &mut AppConfig, task_name: &str) {
    let selection = get_selection_from_enum::<Command>(Some("choose a command for action"));
    match Command::from_str(&selection).unwrap() {
        Command::Move => add_move_action(config, task_name),
        Command::Copy => add_copy_action(config, task_name),
        Command::Delete => add_delete_action(config, task_name),
        Command::Unzip => add_unzip_action(config, task_name),
    }
}

/// Delete 명령어용 Action 추가
fn add_delete_action(config: &mut AppConfig, task_name: &str) {
    if let Some(task) = config.get_task_mut(task_name) {
        if let Some(target_file) = pick_target_file(&task.target_files, "choose a target file to delete") {
            let delete_action = Action::delete_action(target_file);
            task.actions.push(delete_action);
            if let Err(e) = config.save() {
                println!("save error: {}", e);
            }
        }
    }
}

/// Unzip 명령어용 Action 추가
fn add_unzip_action(config: &mut AppConfig, task_name: &str) {
    if let Some(task) = config.get_task_mut(task_name) {
        if let Some(file) = pick_target_file(&task.target_files, "압축을 풀 파일을 선택하세요") {
            let unzip_action = Action {
                target_file: TargetFile {
                    name: file.name.clone(),
                    path: file.path.clone(),
                },
                destination: None,
                command: Command::Unzip,
            };
            task.actions.push(unzip_action);
            if let Err(e) = config.save() {
                println!("save error: {}", e);
            }
        }
    }
}

/// 프로젝트 목록을 보여주고 선택한 프로젝트의 히스토리를 확인하는 예시
fn show_project_history_menu(config: &mut AppConfig, task_name: &str) {
    // Task 가져오기
    if let Some(task) = config.get_task_mut(task_name) {
        // 프로젝트가 하나도 없으면 종료
        if task.projects.is_empty() {
            println!("등록된 프로젝트가 없습니다.");
            return;
        }

        // 프로젝트 경로 목록 (메뉴용)
        let mut project_paths: Vec<String> = task.projects.iter()
            .map(|proj| format!("{} ({:?})", proj.path, proj.vcs_type))
            .collect();
        project_paths.push("Back".to_string());

        // 프로젝트 선택
        let selected_project = get_selection_from_str(
            project_paths,
            Some("히스토리를 확인할 프로젝트를 선택하세요"),
        );

        if selected_project == "Back" {
            return;
        }

        // 실제 Project 찾기
        // selected_project는 "경로 (VcsType)" 형식이므로, 경로 부분만 떼어내서 검색
        // 여기서는 간단하게 문자열 Split 후 맨 앞의 경로 부분으로만 매칭
        let path_part = selected_project.split(" (").next().unwrap_or("");
        if let Some(chosen_proj) = task.projects.iter().find(|p| p.path == path_part) {
            // 프로젝트 히스토리 확인 로직
            show_project_history(chosen_proj);
        } else {
            println!("프로젝트를 찾을 수 없습니다.");
        }
    } else {
        println!("해당 Task가 존재하지 않습니다.");
    }
}

fn show_project_history(project: &Project) {
    match project.vcs_type {
        VcsType::Git => {
            println!(
                "Git 프로젝트 [{}]의 히스토리를 불러오고 있습니다...",
                project.path
            );
            interactive_project_history(project)
        }
        VcsType::Svn => {
            println!(
                "SVN 프로젝트 [{}]의 히스토리를 불러오고 있습니다...",
                project.path
            );
            interactive_project_history(project)
        }
        VcsType::Unknown => {
            println!("이 프로젝트는 VCS 정보가 없습니다: {}", project.path);
            interactive_project_history(project)
        }
    }
    println!("히스토리 조회를 완료했습니다.");
}

fn run_vcs_command(project_path: &str, command: &str, args: &[&str]) -> std::io::Result<String> {
    use std::process::Command;

    let output = Command::new(command)
        .args(args)
        .current_dir(project_path) // 특정 프로젝트 경로에서 실행
        .output()?;

    // UTF-8로 가정하고 문자열 변환
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// TargetFile 중 하나를 대화형으로 선택
fn pick_target_file<'a>(files: &'a [TargetFile], prompt: &str) -> Option<&'a TargetFile> {
    if files.is_empty() {
        println!("등록된 파일이 없습니다.");
        return None;
    }
    let file_names: Vec<String> = files.iter().map(|f| f.name.clone()).collect();
    let selected_name = get_selection_from_str(file_names, Some(prompt));
    files.iter().find(|f| f.name == selected_name)
}

/// Copy 명령어용 Action 추가
fn add_copy_action(config: &mut AppConfig, task_name: &str) {
    if let Some(task) = config.get_task_mut(task_name) {
        if let Some(file) = pick_target_file(&task.target_files, "복사할 파일을 선택하세요") {
            let copy_action = Action {
                target_file: TargetFile {
                    name: file.name.clone(),
                    path: file.path.clone(),
                },
                destination: None,
                command: Command::Copy,
            };
            task.actions.push(copy_action);
            if let Err(e) = config.save() {
                println!("save error: {}", e);
            }
        }
    }
}

/// Move 명령어용 Action 추가
fn add_move_action(config: &mut AppConfig, task_name: &str) {
    if let Some(task) = config.get_task_mut(task_name) {
        if let Some(file) = pick_target_file(&task.target_files, "이동할 파일을 선택하세요") {
            let destination = Input::new()
                .with_prompt("Enter the file path to move the file to")
                .interact_text()
                .unwrap();

            let move_action = Action {
                target_file: TargetFile {
                    name: file.name.clone(),
                    path: file.path.clone(),
                },
                destination: Some(destination),
                command: Command::Move,
            };
            task.actions.push(move_action);
            if let Err(e) = config.save() {
                println!("save error: {}", e);
            }
        }
    }
}

/// TargetFile 추가
fn add_target_file(config: &mut AppConfig, task_name: &str) {
    if let Some(task) = config.get_task_mut(task_name) {
        let options = vec!["Input Manually".to_string(), "Input By File Dialog".to_string(), "Cancel".to_string()];
        let selection = get_selection_from_str(options, Some("choose a file path operation"));
        match selection.as_str() {
            "Input Manually" => {
                let file_path: String = Input::new()
                    .with_prompt("Enter the file path")
                    .interact_text()
                    .unwrap();
                let file_name = Path::new(&file_path).file_name().unwrap().to_str().unwrap();
                let target_file = TargetFile {
                    name: file_name.to_string(),
                    path: file_path,
                };
                task.target_files.push(target_file);
                if let Err(e) = config.save() {
                    println!("save error: {}", e);
                }
            },
            "Input By File Dialog" => {
                if let Some(file_path) = FileDialog::new()
                    .add_filter("All Files", &["*"])
                    .pick_file()
                {
                    let file_name = Path::new(&file_path).file_name().unwrap().to_str().unwrap();
                    let target_file = TargetFile {
                        name: file_name.to_string(),
                        path: file_path.display().to_string(),
                    };
                    task.target_files.push(target_file);
                    if let Err(e) = config.save() {
                        println!("save error: {}", e);
                    }
                }
            },
            "Cancel" => {
                // 취소
            },
            _ => {}
        }
    }
}

/// Git용 히스토리 목록(커밋 목록)을 가져오고, 각 항목(커밋)을 선택하게 한 뒤 변경 파일 정보를 보여주는 예시
fn interactive_git_history(project_path: &str) -> std::io::Result<()> {
    // 1) 히스토리 조회: git log --oneline
    let log_output = run_vcs_command(project_path, "git", &["log", "--oneline"])?;

    // 2) 각 줄이 커밋 하나에 해당하므로, 커밋 목록을 벡터로 분리
    let commits: Vec<&str> = log_output.lines().collect();
    if commits.is_empty() {
        println!("조회된 Git 커밋이 없습니다.");
        return Ok(());
    }

    // 3) 대화형 Select로 커밋을 선택
    //    예) "abcd123 Add new feature"
    let mut select_items = commits.clone();
    select_items.push("뒤로 가기");
    let selected = Select::new()
        .with_prompt("조회할 Git 커밋을 선택하세요")
        .items(&select_items)
        .default(0)
        .interact().unwrap();

    // 사용자가 '뒤로 가기'를 선택하면 종료
    if select_items[selected] == "뒤로 가기" {
        return Ok(());
    }

    let chosen_line = select_items[selected];
    // 커밋 해시 추출 (Git oneline: 맨 앞 부분이 해시)
    // 예) "abcd123 Add new feature"에서 맨 첫 단어를 해시로 가정
    let commit_hash = chosen_line.split_whitespace().next().unwrap_or("");

    // 4) 변경 파일 정보 조회: git show --name-only <commit_hash>
    let show_output = run_vcs_command(project_path, "git", &["show", "--name-only", commit_hash])?;
    // name-only 옵션을 사용하면 변경된 파일 목록이 마지막에 나옵니다.
    // 필요하다면 추가 파싱을 진행 가능

    println!("선택하신 커밋 [{}]에서 변경된 파일 정보:", commit_hash);
    println!("{}", show_output);

    Ok(())
}

/// SVN용 히스토리 목록(리비전 목록)을 가져오고, 각 항목(리비전)을 선택하게 한 뒤 변경 파일 정보를 보여주는 예시
fn interactive_svn_history(project_path: &str) -> std::io::Result<()> {
    // 1) 리비전 목록 조회: svn log -q
    //    -q 옵션: 불필요한 상세 내용을 생략하고 리비전, 작성자, 날짜만 간략히 출력
    let log_output = run_vcs_command(project_path, "svn", &["log", "-q"])?;

    // 2) 리비전 목록 파싱
    //    일반적으로 svn log -q 결과는 아래와 비슷한 형식으로 나오며,
    //    "------------------------------------------------------------------------"
    //    "r123 | user | 2023-10-10 12:34:56 +0900 (Tue, 10 Oct 2023)"
    //    "------------------------------------------------------------------------"
    //    와 유사합니다. 여기서는 단순히 "r123"을 추출하는 방식으로 대화형 목록을 구성합니다.
    let mut revisions = Vec::new();
    for line in log_output.lines() {
        // 예: r123 | user ...
        if line.starts_with('r') && line.contains(" | ") {
            revisions.push(line.to_string());
        }
    }

    if revisions.is_empty() {
        println!("조회된 SVN 리비전이 없습니다.");
        return Ok(());
    }

    // 3) 사용자에게 리비전 선택
    revisions.push("뒤로 가기".to_string());
    let selected = Select::new()
        .with_prompt("조회할 SVN 리비전을 선택하세요")
        .items(&revisions)
        .default(0)
        .interact().unwrap();

    if revisions[selected] == "뒤로 가기" {
        return Ok(());
    }

    // r123 | user ...
    let chosen_line = &revisions[selected];
    // 맨 앞의 r123 숫자를 추출(공백 전까지)
    let rev_str = chosen_line.split_whitespace().next().unwrap_or("");
    // "r123"에서 숫자만 추출
    let revision_num = rev_str.trim_start_matches('r');

    // 4) 변경 파일 조회: svn log -v -r <revision> 
    //    -v 옵션: 변경된 파일(Added, Modified, Deleted 등) 정보를 표시
    let show_output = run_vcs_command(project_path, "svn", &["log", "-v", "-r", revision_num])?;
    println!("선택하신 리비전 [{}]에서 변경된 파일 정보:", rev_str);
    println!("{}", show_output);

    Ok(())
}

/// 프로젝트의 유형에 맞춰 히스토리 확인 후, 커밋/리비전 선택과 변경 파일 정보를 조회하도록 연결해주는 함수
fn interactive_project_history(project: &Project) {
    match project.vcs_type {
        VcsType::Git => {
            println!("Git 프로젝트 [{}]의 히스토리를 조회합니다...", project.path);
            if let Err(e) = interactive_git_history(&project.path) {
                println!("Git 히스토리 조회 중 오류 발생: {}", e);
            }
        }
        VcsType::Svn => {
            println!("SVN 프로젝트 [{}]의 히스토리를 조회합니다...", project.path);
            if let Err(e) = interactive_svn_history(&project.path) {
                println!("SVN 히스토리 조회 중 오류 발생: {}", e);
            }
        }
        VcsType::Unknown => {
            println!("알 수 없는 유형의 프로젝트입니다: {}", project.path);
        }
    }
}

fn exit() {
    println!("Exiting Easy Patcher...");
    std::process::exit(0);
}