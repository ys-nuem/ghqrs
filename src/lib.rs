extern crate walkdir;
extern crate regex;
extern crate url;

use std::process::Command;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug)]
struct Repository {
  root: String,
  path: String,
}

pub fn command_get(projects: Vec<String>, skip_pull: bool, shallow: bool) -> i32 {
  for project in projects {
    let (protocol, base_url, user, repo) = parse_token(project, None);
    let url = url::Url::parse(&format!("{}://{}/{}/{}.git", protocol, base_url, user, repo))
      .unwrap();

    let mut dest = PathBuf::from(&get_local_repos_roots()[0]);
    dest.push(base_url);
    dest.push(user);
    dest.push(repo);

    git_clone_or_pull(url, dest.as_path(), skip_pull, shallow);
  }
  0
}

pub fn command_list(exact: bool, fullpath: bool, unique: bool, query: Option<String>) -> i32 {
  let filter: Box<Fn(&str) -> bool> = {
    if let Some(query) = query {
      if exact {
        Box::new(move |s: &str| s == query)
      } else {
        Box::new(move |s: &str| s.contains(&query))
      }
    } else {
      Box::new(|_| true)
    }
  };

  for repo in get_local_repositories(filter) {
    if fullpath {
      let mut repo_path = PathBuf::from(repo.root);
      repo_path.push(repo.path);
      println!("{}", repo_path.display());
    } else if unique {
      let repo_name = Path::new(&repo.path)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();
      println!("{}", repo_name);
    } else {
      println!("{}", repo.path);
    }
  }

  0
}

pub fn command_root(all: bool) -> i32 {
  let roots = get_local_repos_roots();
  if all {
    for root in roots {
      println!("{}", root);
    }
  } else {
    println!("{}", roots[0]);
  }
  0
}

fn git_clone_or_pull(url: url::Url, dest: &Path, skip_pull: bool, shallow: bool) {
  if dest.exists() {
    if !skip_pull {
      git_pull(dest);
    }
  } else {
    git_clone(url, dest, shallow);
  }
}

#[allow(unreachable_code)]
fn git_clone(url: url::Url, dest: &Path, shallow: bool) {
  println!("clone: {:?} -> {:?} ({})",
           url,
           dest,
           if shallow { "Shallow" } else { "" });
  return;

  let mut args = vec!["clone", url.as_str(), dest.to_str().unwrap()];
  if shallow {
    args.extend(&["--depth", "1"]);
  }

  let output = Command::new("git")
    .args(&args[..])
    .output()
    .expect("failed to clone repository");
  if !output.status.success() {
    panic!("git clone failed");
  }
}

struct ScopeExit<F: FnMut()> {
  caller: F,
}

impl<F: FnMut()> ScopeExit<F> {
  fn new(c: F) -> ScopeExit<F> {
    ScopeExit { caller: c }
  }
}

impl<F: FnMut()> Drop for ScopeExit<F> {
  fn drop(&mut self) {
    (self.caller)();
  }
}

#[allow(unreachable_code)]
fn git_pull(dest: &Path) {
  println!("pull: {:?}", dest);
  return;

  let curr_dir = std::env::current_dir().unwrap().to_str().unwrap().to_owned();
  std::env::set_current_dir(dest).unwrap();
  let scope1 = ScopeExit::new(|| {
    std::env::set_current_dir(curr_dir.clone()).unwrap();
  });

  let output = Command::new("git")
    .args(&["pull"])
    .output()
    .expect("failed to clone repository");
  if !output.status.success() {
    panic!("git pull failed");
  }
}

fn git_config(key: &str) -> String {
  let output = Command::new("git")
    .args(&["config", "--path", "--null", "--get-all", key])
    .output()
    .expect("failed to execute git");
  let len = output.stdout.len();
  String::from_utf8(Vec::from(&output.stdout[0..len - 1])).unwrap()
}

fn get_local_repositories(filter: Box<Fn(&str) -> bool>) -> Vec<Repository> {
  let mut dst = Vec::new();

  let roots = get_local_repos_roots();
  for root in roots {
    for entry in WalkDir::new(&root).follow_links(true).into_iter().filter_map(|e| e.ok()) {
      let path = format!("{}", entry.path().display()).replace(&format!("{}/", root), "");
      if entry.depth() == 3 {

        let entry = vec![".git", ".svn", ".hg", "_darcs"].into_iter().find(|&e| {
          let mut buf = PathBuf::from(format!("{}", entry.path().display()));
          if !filter(buf.file_name().unwrap().to_str().unwrap()) {
            return false;
          }
          buf.push(e);
          buf.exists()
        });

        if entry.is_some() {
          dst.push(Repository {
            root: root.clone(),
            path: path,
          });
        }
      }
    }
  }

  dst
}


fn get_local_repos_roots() -> Vec<String> {
  let mut local_repo_roots;

  let env_root: String = std::env::var("GHQ_ROOT").unwrap_or("".to_owned());
  if env_root == "" {
    local_repo_roots = vec![git_config("ghq.root")];
  } else {
    local_repo_roots = env_root.split(":").map(|s| s.to_owned()).collect();
  }

  if local_repo_roots.len() == 0 {
    let mut ghq_path = std::env::home_dir().unwrap();
    ghq_path.push(".ghq");
    local_repo_roots.push(format!("{}", ghq_path.display()));
  }

  assert!(local_repo_roots.len() >= 1);

  local_repo_roots
}

fn parse_token(project: String, root: Option<&str>) -> (String, String, String, String) {
  let (protocol, base_url, user, repo): (String, String, String, String);
  match url::Url::parse(&project) {
    Ok(url) => {
      protocol = url.scheme().to_owned();
      base_url = url.host_str().unwrap().to_owned();

      let paths: Vec<_> = url.path_segments().unwrap().map(ToOwned::to_owned).collect();
      user = paths[0].clone();
      repo = paths[1].trim_right_matches(".git").to_owned();
    }
    Err(_) => {
      protocol = "https".to_owned();
      let paths: Vec<String> =
        project.split(std::path::MAIN_SEPARATOR).map(ToOwned::to_owned).collect();
      if paths.len() > 1 && (paths[0] == "." || paths[0] == "..") {
        let mut buf = std::env::current_dir().unwrap().to_path_buf();
        if paths[0] == ".." {
          buf.pop();
        }
        for path in &paths[1..] {
          buf.push(path);
        }

        let root = root.map(ToOwned::to_owned)
          .unwrap_or(get_local_repos_roots().into_iter().nth(0).unwrap());

        let path = std::fs::canonicalize(&buf)
          .unwrap()
          .to_str()
          .unwrap()
          .trim_left_matches(&format!("{}{}", root, std::path::MAIN_SEPARATOR))
          .to_owned();

        return parse_token(path, None);
      } else {
        if paths.len() == 3 {
          base_url = paths[0].clone();
          user = paths[1].clone();
          repo = paths[2].clone();
        } else if paths.len() == 2 {
          base_url = "github.com".to_owned();
          user = paths[0].clone();
          repo = paths[1].clone();
        } else {
          base_url = "github.com".to_owned();
          user = project.clone();
          repo = project.clone();
        }
      }
    }
  };

  (protocol, base_url, user, repo)
}

#[cfg(test)]
mod test_parse_token {
  extern crate tempdir;
  use super::parse_token;
  use std;

  #[test]
  fn user_project() {
    let input = "hoge/fuga";
    let output = parse_token(input.to_owned(), None);
    assert_eq!(output,
               ("https".to_owned(), "github.com".to_owned(), "hoge".to_owned(), "fuga".to_owned()));
  }

  #[test]
  fn domain_user_project() {
    let input = "github.com/hoge/fuga";
    let output = parse_token(input.to_owned(), None);
    assert_eq!(output,
               ("https".to_owned(), "github.com".to_owned(), "hoge".to_owned(), "fuga".to_owned()));
  }

  #[test]
  fn only_project_name() {
    let input = "fuga";
    let output = parse_token(input.to_owned(), None);
    assert_eq!(output,
               ("https".to_owned(), "github.com".to_owned(), "fuga".to_owned(), "fuga".to_owned()));
  }

  #[test]
  fn repository_url() {
    let input = "https://gitlab.com/funga-/pecopeco.git";
    let output = parse_token(input.to_owned(), None);
    assert_eq!(output,
               ("https".to_owned(),
                "gitlab.com".to_owned(),
                "funga-".to_owned(),
                "pecopeco".to_owned()));
  }

  #[test]
  fn relative_path_1() {
    let tmp_dir = tempdir::TempDir::new("test1").unwrap();

    let mut wd = tmp_dir.path().to_path_buf();
    wd.push("github.com");
    wd.push("hoge");
    wd.push("fuga");
    std::fs::create_dir_all(wd.to_str().unwrap()).unwrap();

    wd.pop();
    std::env::set_current_dir(wd.as_path()).unwrap();

    let input = "./fuga";
    let output = parse_token(input.to_owned(), tmp_dir.path().to_str());
    assert_eq!(output,
               ("https".to_owned(), "github.com".to_owned(), "hoge".to_owned(), "fuga".to_owned()));
  }

  #[test]
  fn relative_path_2() {
    let tmp_dir = tempdir::TempDir::new("test2").unwrap();

    let mut wd = tmp_dir.path().to_path_buf();
    wd.push("github.com");
    wd.push("hoge");
    wd.push("fuga");
    std::fs::create_dir_all(wd.to_str().unwrap()).unwrap();

    std::env::set_current_dir(wd.as_path()).unwrap();

    let input = "../fuga";
    let output = parse_token(input.to_owned(), tmp_dir.path().to_str());
    assert_eq!(output,
               ("https".to_owned(), "github.com".to_owned(), "hoge".to_owned(), "fuga".to_owned()));
  }
}
