use std::collections::HashSet;
use dialoguer::{MultiSelect, Confirm, Input, Select};
use chrono::Local;
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
use git::{ChangedFile, CommitInfo};
use svn::RevisionInfo;
/*TODO
1. excute run 하면 각 프로젝트 별로 SVN 히스토리 불러오기
2. 프로젝트별로 패치할 히스토리 체크박스로 선택
3. 아무 히스토리도 선택하지 않은 프로젝트 제외하고 선택한 히스토리를 통해 변경되야할 파일 목록 출력(confirm, cancel)
4. confirm 하면 프로젝트별로 build_file_path 순회해서 war파일인 경우 unzip(이때 unzip할 폴더명이 해당 경로에 존재하면 삭제 후 unzip)
4. 더이상 프로젝트가 존재하지 않으면 execute_task_with_timestamp 함수 호출하는데 지금은 시간폴더 내 동일한 위치에 생성하지만 각 프로젝트명 별로 폴더를 만들어서 해당 폴더 내에 복사하는 로직으로 변경
5. 각 프로젝트 폴더 내에 아까 선택한 히스토리 정보가 담긴 파일을 추가
6. 각 프로젝트 별로 리눅스에서 실행가능한 bash스크립트 파일을 추가하는데 
    실행하면 "patch, target setting, exit" 옵션이 있고  target setting을 통해 패치할 경로를 추가하게 함(여러 경로 추가 가능)
    patch를 실행하면 저장된 타겟폴더 내에서 히스토리 정보에 맞춰 파일들을 패치하기전에 백업을 진행 > 패치할 파일들을 전부 스크립트 경로와 동일한 위치에 bak_날짜시간 폴더를 만들어서 백업해둠
    백업이 완료되면 히스토리 정보에 맞춰 파일을 추가/덮어씌우기/삭제를 진행 
*/
const CONFIG_FILE_PATH: &str = "config.json";

#[derive(Debug, Serialize, Deserialize)]
struct Task {
    name: String,
    target_files: Vec<TargetFile>,
    actions: Vec<Action>,
    projects: Vec<Project>,
    output_dir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Project {
    path: String,
    vcs_type: VcsType,
    build_file_path: Option<String>,
}

#[derive(Debug)]
pub enum ProjectHistory {
    GitHistory(String, Vec<CommitInfo>),
    SvnHistory(String, Vec<RevisionInfo>),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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
            output_dir: None,
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
    Setting,
    AddTargetFile,
    AddProject,
    ActionList,
    ProjectList,
    DeleteTask,
    Back,
}

#[derive(Debug, EnumIter, EnumString)]
enum TaskSettingMenu {
    OutputDir,
    BuildFilePath,
    Back,
}

fn main() {
    let mut config = AppConfig::load();

    loop {
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
    run_svn_patching_flow(config.get_task_mut(&selected_task).unwrap().projects.as_slice()).expect("TODO: panic message");
    execute_task_with_timestamp(&config.get_task(&selected_task).unwrap()).unwrap();
    execute_action_list(config, &selected_task);
}

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
                break;
            },
        }
    }
}

fn run_task_setting_menu(config: &mut AppConfig, task_name: &str) {
    loop {
        let selection = get_selection_from_enum::<TaskSettingMenu>(None);
        match TaskSettingMenu::from_str(&selection).unwrap() {
            TaskSettingMenu::OutputDir => {
                if let Some(file_path) = FileDialog::new()
                    .add_filter("All Files", &["*"])
                    .pick_folder()
                {
                    let output_dir = Path::new(&file_path).to_str().unwrap();
                    if let Some(task) = config.get_task_mut(task_name) {
                        task.output_dir = Some(output_dir.parse().unwrap());
                    }
                    if let Err(e) = config.save() {
                        println!("save error: {}", e);
                    }
                }
            },
            TaskSettingMenu::BuildFilePath => {
                let mut project_names = config.get_task_mut(task_name)
                    .unwrap().projects.iter()
                    .map(|proj| proj.path.clone() + "[" + {proj.build_file_path.as_ref().unwrap_or(&String::from("Not set"))} + "]")
                    .collect::<Vec<String>>();
                project_names.push("Back".to_string());
                let selection = get_selection_from_str(project_names, Some("choose a task"));
                if selection == "Back" {
                    return;
                } else {
                    if let Some(file_path) = FileDialog::new()
                        .add_filter("All Files", &["*"])
                        .pick_file()
                    {
                        let build_file_path = Path::new(&file_path).to_str().unwrap();
                        if let Some(task) = config.get_task_mut(task_name) {
                            task.projects.iter_mut().find(|proj| proj.path == selection.split("[").next().unwrap() )
                                .unwrap().build_file_path = Some(build_file_path.parse().unwrap());
                        }
                        if let Err(e) = config.save() {
                            println!("save error: {}", e);
                        }
                    }
                }
            }
            _ => {
                println!("Not implemented yet");
            }
        }
    }
}

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
            TaskMenu::Setting => run_task_setting_menu(config, &selection),
            TaskMenu::AddTargetFile => add_target_file(config, &selection),
            TaskMenu::AddProject => add_project(config, &selection),
            TaskMenu::ActionList => action_list(config, &selection),
            TaskMenu::ProjectList => show_project_history_menu(config, &selection),
            TaskMenu::DeleteTask => {
                delete_task(config, &selection);
                break;
            },
            TaskMenu::Back => {
                break;
            },
        }
    }
}

fn add_project(config: &mut AppConfig, task_name: &str) {
    if let Some(task) = config.get_task_mut(task_name) {
        println!("프로젝트 폴더를 선택하세요:");
        if let Some(folder_path) = FileDialog::new().pick_folder() {
            let folder_str = folder_path.display().to_string();
            let git_path = folder_path.join(".git");
            let svn_path = folder_path.join(".svn");
            let vcs_type = if git_path.exists() {
                VcsType::Git
            } else if svn_path.exists() {
                VcsType::Svn
            } else {
                VcsType::Unknown
            };

            let new_project = Project {
                path: folder_str,
                vcs_type,
                build_file_path: None
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

fn delete_task(config: &mut AppConfig, task_name: &str) {
    config.tasks.retain(|task| task.name != task_name);
    if let Err(e) = config.save() {
        println!("delete_task save error: {}", e);
    }
}
fn execute_task_with_timestamp(task: &Task) -> std::io::Result<()> {
    use chrono::Local;
    use std::fs;
    use std::path::{Path, PathBuf};

    // 1. 현재 날짜와 시간을 받아 폴더 이름을 생성합니다.
    let now = Local::now();
    let folder_name = now.format("%Y%m%d_%H%M%S").to_string();

    // 2. Task에 설정된 output_dir이 있는지 확인 후, 새로운 폴더 경로를 구성합니다.
    let base_output_dir = match &task.output_dir {
        Some(dir) => dir,
        None => {
            println!("output_dir가 설정되어 있지 않습니다.");
            return Ok(());
        }
    };
    let new_folder_path = Path::new(base_output_dir).join(folder_name);

    // 3. 새 폴더를 생성합니다.
    fs::create_dir_all(&new_folder_path)?;

    // 4. Task에 포함된 모든 Project를 확인하고, build_file_path가 있으면 복사합니다.
    for project in &task.projects {
        if let Some(ref build_path) = project.build_file_path {
            let src_path = PathBuf::from(build_path);
            if src_path.is_file() {
                // 원본 파일 이름을 구해서, 새로 만든 폴더 안에 같은 이름으로 복사합니다.
                if let Some(file_name) = src_path.file_name() {
                    let dest_path = new_folder_path.join(file_name);
                    fs::copy(&src_path, &dest_path)?;
                    println!("파일 복사 완료: {:?} -> {:?}", src_path, dest_path);
                }
            } else {
                println!("프로젝트의 build_file_path가 올바른 파일이 아닙니다: {}", build_path);
            }
        } else {
            println!("프로젝트에 build_file_path가 설정되어 있지 않습니다: {}", project.path);
        }
    }
    println!("작업이 완료되었습니다: {:?}", new_folder_path);
    Ok(())
}

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

fn action_list(config: &mut AppConfig, task_name: &str) {
    if let Some(task) = config.get_task(task_name) {
        for (index, action) in task.actions.iter().enumerate() {
            println!("{}: {:?}", index, action);
        }
    }
}

fn add_action(config: &mut AppConfig, task_name: &str) {
    command_list(config, task_name);
}

fn get_selection_from_enum<T: IntoEnumIterator + Debug>(prompt: Option<&str>) -> String {
    let items: Vec<String> = T::iter().map(|item| format!("{:?}", item)).collect();
    let mut select = Select::new().items(&items).default(0);

    if let Some(text) = prompt {
        select = select.with_prompt(text);
    }
    let selection = select.interact().unwrap();
    items[selection].clone()
}

fn get_selection_from_str(items: Vec<String>, prompt: Option<&str>) -> String {
    let mut select = Select::new().items(&items).default(0);

    if let Some(text) = prompt {
        select = select.with_prompt(text);
    }
    let selection = select.interact().unwrap();
    items[selection].clone()
}

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

fn delete_file(target_file: &String) {
    let path = Path::new(target_file);
    if let Err(e) = fs::remove_file(path) {
        println!("파일 삭제 실패: {}", e);
    } else {
        println!("파일이 성공적으로 삭제되었습니다.");
    }
}

fn unzip_file(zip_path: &str, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file_in_zip = archive.by_index(i)?;
        let out_path = Path::new(output_dir);

        if file_in_zip.is_dir() {
            fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            let mut out_file = File::create(&out_path)?;
            std::io::copy(&mut file_in_zip, &mut out_file)?;
        }
    }
    Ok(())
}

fn command_list(config: &mut AppConfig, task_name: &str) {
    let selection = get_selection_from_enum::<Command>(Some("choose a command for action"));
    match Command::from_str(&selection).unwrap() {
        Command::Move => add_move_action(config, task_name),
        Command::Copy => add_copy_action(config, task_name),
        Command::Delete => add_delete_action(config, task_name),
        Command::Unzip => add_unzip_action(config, task_name),
    }
}

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

fn show_project_history_menu(config: &mut AppConfig, task_name: &str) {
    if let Some(task) = config.get_task_mut(task_name) {
        if task.projects.is_empty() {
            println!("등록된 프로젝트가 없습니다.");
            return;
        }

        let mut project_paths: Vec<String> = task.projects.iter()
            .map(|proj| format!("{} ({:?})", proj.path, proj.vcs_type))
            .collect();
        project_paths.push("Back".to_string());

        let selected_project = get_selection_from_str(
            project_paths,
            Some("히스토리를 확인할 프로젝트를 선택하세요"),
        );

        if selected_project == "Back" {
            return;
        }

        let path_part = selected_project.split(" (").next().unwrap_or("");
        if let Some(chosen_proj) = task.projects.iter().find(|p| p.path == path_part) {
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

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn pick_target_file<'a>(files: &'a [TargetFile], prompt: &str) -> Option<&'a TargetFile> {
    if files.is_empty() {
        println!("등록된 파일이 없습니다.");
        return None;
    }
    let file_names: Vec<String> = files.iter().map(|f| f.name.clone()).collect();
    let selected_name = get_selection_from_str(file_names, Some(prompt));
    files.iter().find(|f| f.name == selected_name)
}

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
            },
            _ => {}
        }
    }
}

fn interactive_git_history(project_path: &str) -> std::io::Result<()> {
    let log_output = run_vcs_command(project_path, "git", &["log", "--oneline"])?;

    let commits: Vec<&str> = log_output.lines().collect();
    if commits.is_empty() {
        println!("조회된 Git 커밋이 없습니다.");
        return Ok(());
    }

    let mut select_items = commits.clone();
    select_items.push("뒤로 가기");
    let selected = Select::new()
        .with_prompt("조회할 Git 커밋을 선택하세요")
        .items(&select_items)
        .default(0)
        .interact().unwrap();

    if select_items[selected] == "뒤로 가기" {
        return Ok(());
    }

    let chosen_line = select_items[selected];
    let commit_hash = chosen_line.split_whitespace().next().unwrap_or("");

    // 4) 변경 파일 정보 조회: git show --name-only <commit_hash>
    let show_output = run_vcs_command(project_path, "git", &["show", "--name-only", commit_hash])?;

    println!("선택하신 커밋 [{}]에서 변경된 파일 정보:", commit_hash);
    println!("{}", show_output);

    Ok(())
}

fn interactive_svn_history(project_path: &str) -> std::io::Result<()> {
    let log_output = run_vcs_command(project_path, "svn", &["log", "-q"])?;

    let mut revisions = Vec::new();
    for line in log_output.lines() {
        if line.starts_with('r') && line.contains(" | ") {
            revisions.push(line.to_string());
        }
    }

    if revisions.is_empty() {
        println!("조회된 SVN 리비전이 없습니다.");
        return Ok(());
    }

    revisions.push("뒤로 가기".to_string());
    let selected = Select::new()
        .with_prompt("조회할 SVN 리비전을 선택하세요")
        .items(&revisions)
        .default(0)
        .interact().unwrap();

    if revisions[selected] == "뒤로 가기" {
        return Ok(());
    }

    let chosen_line = &revisions[selected];
    let rev_str = chosen_line.split_whitespace().next().unwrap_or("");
    let revision_num = rev_str.trim_start_matches('r');

    let show_output = run_vcs_command(project_path, "svn", &["log", "-v", "-r", revision_num])?;
    println!("선택하신 리비전 [{}]에서 변경된 파일 정보:", rev_str);
    println!("{}", show_output);

    Ok(())
}

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


/// (1) 실행 시 각 프로젝트별 SVN 히스토리를 불러와서, 원하는 리비전(혹은 여러 리비전)을 체크박스로 선택하게 하는 예시
/// (2) 사용자가 선택하지 않은 프로젝트는 제외
/// (3) 선택된 히스토리를 기반으로 변경될 파일 목록을 출력 후, 사용자에게 적용 여부를 묻는다 (Confirm)
pub fn run_svn_patching_flow(projects: &[Project]) -> std::io::Result<()> {
    // 1. 각 프로젝트에서 Git 히스토리 불러오기
    let mut project_histories: Vec<ProjectHistory> = Vec::new();
    for proj in projects {
        if proj.vcs_type == VcsType::Git {
            println!("프로젝트 [{}]의 Git 히스토리를 불러오는 중...", proj.path);
            let revisions = git::get_commit_history(&proj.path);
            match revisions {
                Ok(commit_list) => {
                    // commit_list는 Vec<CommitInfo> 이므로 그대로 넣습니다.
                    project_histories.push(ProjectHistory::GitHistory(proj.path.clone(), commit_list));
                }
                Err(e) => {
                    // 에러 처리 로직 (로그 출력 등)
                    eprintln!("히스토리를 가져오는 중 오류가 발생: {:?}", e);
                }
            }
        } else if proj.vcs_type == VcsType::Svn {
            println!("프로젝트 [{}]의 Svn 히스토리를 불러오는 중...", proj.path);
            let revisions = svn::get_svn_history(&proj.path, 10);
            match revisions {
                Ok(revision_info_list) => {
                    // revision_info_list Vec<RevisionInfo> 이므로 그대로 넣습니다.
                    project_histories.push(ProjectHistory::SvnHistory(proj.path.clone(), revision_info_list));
                }
                Err(e) => {
                    // 에러 처리 로직 (로그 출력 등)
                    eprintln!("히스토리를 가져오는 중 오류가 발생: {:?}", e);
                }
            }
            
        } else {
            println!("Git / Svn 프로젝트가 아니므로 패스: {}", proj.path);
        }
    }

    // 2. 프로젝트별로 SVN 리비전을 체크박스로 선택
    let mut selected_histories: Vec<ProjectHistory> = Vec::new();
    for project_history in project_histories {
        match project_history {
            ProjectHistory::GitHistory(proj_path, commits) => {
                println!("=== [{}] 프로젝트 Git 리비전 선택 ===", proj_path);

                if commits.is_empty() {
                    println!("히스토리가 없습니다.");
                    continue;
                }

                // 다중 선택(체크박스) 예시
                // - `items()`에는 표시할 리스트를 넣습니다.
                // - commits[i].clone()을 하기 위해 CommitInfo에 Clone이 필요합니다.
                let selected_revisions = MultiSelect::new()
                    .with_prompt("패치하고자 하는 리비전을 선택하세요 (스페이스바로 체크)")
                    .items(&commits)
                    .interact()
                    .unwrap();

                // 사용자가 체크한 항목만 추려서 새로운 벡터를 구성
                let mut chosen_items = Vec::new();
                for &i in &selected_revisions {
                    chosen_items.push(commits[i].clone());
                }

                if chosen_items.is_empty() {
                    println!("선택된 리비전이 없으므로 [{}] 프로젝트는 패치에서 제외됩니다.", proj_path);
                } else {
                    // 선택된 항목이 있다면 결과를 다시 ProjectHistory로 담아둠
                    selected_histories.push(ProjectHistory::GitHistory(proj_path.clone(), chosen_items));
                }
            }

            ProjectHistory::SvnHistory(proj_path, revision_info_list) => {
                println!("=== [{}] 프로젝트 Svn 리비전 선택 ===", proj_path);

                if revision_info_list.is_empty() {
                    println!("히스토리가 없습니다.");
                    continue;
                }

                // SVN 리비전 목록 체크박스 예시
                let selected_revisions = MultiSelect::new()
                    .with_prompt("패치하고자 하는 리비전을 선택하세요 (스페이스바로 체크)")
                    .items(&revision_info_list)
                    .interact()
                    .unwrap();

                let mut chosen_items = Vec::new();
                for &i in &selected_revisions {
                    chosen_items.push(revision_info_list[i].clone());
                }

                if chosen_items.is_empty() {
                    println!("선택된 리비전이 없으므로 [{}] 프로젝트는 패치에서 제외됩니다.", proj_path);
                } else {
                    // 선택된 항목이 있다면 결과를 다시 ProjectHistory로 담아둠
                    selected_histories.push(ProjectHistory::SvnHistory(proj_path.clone(), chosen_items));
                }
            }
        }
    }

    // 3. 선택된 리비전별로 '가상의 변경 파일 목록'을 출력하고, 실제 패치 적용을 할지 확인
    if selected_histories.is_empty() {
        println!("적용할 리비전을 선택한 프로젝트가 없습니다. 작업을 종료합니다.");
        return Ok(());
    }

    println!("=== 선택된 히스토리에 따른 가상 변경 파일 목록 ===");
    let mut project_patch_infos: Vec<(String, HashSet<(String, String)>)> = Vec::new();
    for selected_history in &selected_histories {
        match selected_history {
            ProjectHistory::GitHistory(proj_path, commits) => {
                println!("=== [{}] 프로젝트 Git 가상 변경 파일 목록 ===", proj_path);
                for commit in commits {
                    if let affected_files =  git::get_changed_files_with_status(proj_path, &commit.commit_id).unwrap() {
                        let mut affected_files_set: HashSet<(String, String)> = HashSet::new();
                        for file in affected_files {
                            if affected_files_set.insert((file.change_type.to_string(), file.path.clone())) {
                                println!("  {}", file);
                            }
                        }
                        project_patch_infos.push((proj_path.clone(), affected_files_set));
                    };
                }
            }

            ProjectHistory::SvnHistory(proj_path, revision_info_list) => {
                println!("=== [{}] 프로젝트 Svn 가상 변경 파일 목록 ===", proj_path);
                for revision_info in revision_info_list {
                    if let affected_files =  svn::get_changed_files_in_revision(proj_path, &revision_info.revision).unwrap() {
                        let mut affected_files_set: HashSet<(String, String)> = HashSet::new();
                        for file in affected_files {
                            if affected_files_set.insert((file.clone(), file.clone())) {
                                println!("  {}", file);
                            }
                        }
                        project_patch_infos.push((proj_path.clone(), affected_files_set));
                    };
                }
            }
            
        }
    }
    
    // 사용자에게 계속 진행할지 확인
    let do_confirm = Confirm::new()
        .with_prompt("위 변경 파일들을 대상으로 패치를 진행하시겠습니까?")
        .default(true)
        .interact().unwrap();

    if !do_confirm {
        println!("사용자가 패치를 취소했습니다.");
        return Ok(());
    }

    // (4) confirm 시 build_file_path가 WAR 파일이면 unzip(폴더가 이미 있으면 삭제 후 unzip) 실행하기
    //     이후, 더이상 프로젝트가 없으면 execute_task_with_timestamp 로직을 호출하고
    //     각 프로젝트별 폴더 생성 후 build_file_path를 복사하는 식으로 작업 수행
    for proj in projects {
        if let Some(build_file) = &proj.build_file_path {
            let build_path = Path::new(build_file);
            if build_path.extension().and_then(|ext| ext.to_str()) == Some("war") {
                // 이미 unzip할 폴더가 있다면 삭제
                let unzip_dir = build_path.parent().unwrap().join("unzip_target");
                if unzip_dir.exists() {
                    fs::remove_dir_all(&unzip_dir)?;
                }
                fs::create_dir_all(&unzip_dir)?;

                // 여기서는 단순히 예시 출력
                println!("WAR 파일을 해제하는 로직을 실행합니다: {:?}", build_file);
                // unzip_file(build_file, unzip_dir.to_str().unwrap())?;
            }
        }
    }

    // 예시로 모든 작업이 끝났다고 가정하고, 프로젝트별 timestamp 디렉토리에 복사
    let now_str = Local::now().format("%Y%m%d_%H%M%S").to_string();
    for proj in projects {
        // 각 프로젝트 이름(폴더명)으로 구성
        let custom_folder = format!("{}_{}", proj.path.replace('/', "_"), now_str);
        println!("=> 프로젝트 전용 폴더 생성 및 복사: {}", custom_folder);
        // 실제로는 output_dir 아래에 프로젝트 이름으로 폴더 만들고, build_file_path 파일 복사 등 수행
    }

    // (5) 선택한 리비전 정보가 담긴 파일 저장(예: txt/json 등)
    //     실제로는 "프로젝트폴더/선택된_리비전들.txt" 등을 만들어 저장하는 로직 추가
    println!("선택된 리비전 정보를 각 프로젝트별 폴더에 기록합니다.");

    // (6) 프로젝트별 리눅스 bash 스크립트 생성
    //     실제 생성 예시 (개념):
    println!("프로젝트별로 patch, target setting, exit 옵션이 포함된 스크립트를 생성합니다.");

    println!("모든 작업이 완료되었습니다.");
    Ok(())
}

fn test() {
    
}