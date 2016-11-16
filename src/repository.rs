use std::path::{Path, PathBuf};
use url::Url;
use error::GhqError;
use vcs;


#[allow(dead_code)]
#[cfg_attr(rustfmt, rustfmt_skip)]
const KNOWN_HOSTS: &'static [(&'static str, usize)] = &[
    ("github.com", 2)
  , ("gist.github.com", 1)
  , ("bitbucket.org", 2)
  , ("gitlab.com", 2)
];


#[derive(Debug)]
pub struct Repository {
  url: Option<Url>,
  host: String,
  path: String,
}

impl Repository {
  pub fn from_local(path: &Path) -> Result<Repository, GhqError> {
    let _ = vcs::detect(path);

    let path = path.to_string_lossy().into_owned();
    let splitted: Vec<_> = path.splitn(3, '/').collect();
    if splitted.len() < 3 {
      return Err("").map_err(Into::into);
    }

    let host = splitted[0].to_owned();
    let path = Vec::from(&splitted[1..]).join("/");

    Ok(Repository {
      url: None,
      host: host,
      path: path,
    })
  }

  pub fn from_remote(s: &str) -> Result<Repository, GhqError> {
    if let Ok(url) = Url::parse(s) {
      let host = url.host_str().ok_or("cannot retrieve host information")?.to_owned();
      let path = url.path().trim_left_matches("/").trim_right_matches(".git").to_owned();
      // TODO: check if given URL is valid

      Ok(Repository {
        url: Some(url),
        host: host,
        path: path,
      })

    } else {
      let path: Vec<_> = s.split("/").collect();
      let (host, path) = match path.len() {
        0 => Err("unsupported pattern to resolve remote URL")?,
        1 => ("github.com".to_owned(), vec![path[0], path[0]]),
        2 => ("github.com".to_owned(), vec![path[0], path[1]]),
        _ => (path[0].to_owned(), Vec::from(&path[1..])),
      };

      let url = Url::parse(&format!("{}://{}/{}.git",
                                    "https",
                                    host,
                                    path.iter().take(2).cloned().collect::<Vec<_>>().join("/")))?;

      Ok(Repository {
        url: Some(url),
        host: host,
        path: path.join("/"),
      })
    }
  }

  pub fn local_path(&self, root: &str) -> PathBuf {
    Path::new(root).join(&self.host).join(&self.path)
  }

  pub fn clone_into(&self, root: &str) -> Result<(), GhqError> {
    if let Some(ref url) = self.url {
      let dest = Path::new(root).join(&self.host).join(&self.path);

      if dest.exists() {
        println!("The target has already existed: {}", dest.display());
        return Ok(());
      }

      println!("clone '{}' into '{}'", url.as_str(), dest.display());
      vcs::Git::clone(url, dest.as_path(), None).map(|_| ()).map_err(Into::into)
    } else {
      Ok(())
    }
  }
}


#[cfg(test)]
mod test_from_remote {
  use super::Repository;

  macro_rules! def_test {
    ($name:ident, $s:expr, $url:expr, $host:expr, $path:expr) => {
      #[test]
      fn $name() {
        let repo = Repository::from_remote($s).unwrap();
        assert_eq!(repo.url.unwrap().as_str(), $url);
        assert_eq!(repo.host, $host);
        assert_eq!(repo.path, $path);
      }
    }
  }

  def_test!(user_project,
            "hoge/fuga",
            "https://github.com/hoge/fuga.git",
            "github.com",
            "hoge/fuga");

  def_test!(domain_user_project,
            "github.com/hoge/fuga",
            "https://github.com/hoge/fuga.git",
            "github.com",
            "hoge/fuga");

  def_test!(only_project_name,
            "fuga",
            "https://github.com/fuga/fuga.git",
            "github.com",
            "fuga/fuga");

  def_test!(repository_url,
            "https://gitlab.com/funga-/pecopeco.git",
            "https://gitlab.com/funga-/pecopeco.git",
            "gitlab.com",
            "funga-/pecopeco");

  def_test!(long_path,
            "github.com/hoge/fuga/foo/a/b/c",
            "https://github.com/hoge/fuga.git",
            "github.com",
            "hoge/fuga/foo/a/b/c");
}
