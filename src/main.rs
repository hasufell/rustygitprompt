use git2::Error;
use std::env;
use colored::*;

fn main() -> std::io::Result<()> {
    let repo = match git2::Repository::discover(env::current_dir()?.as_path()) {
        Ok(r) => r,
        Err(_r) => return Ok(()),
    };

    let r = Repository { repository: repo };

    match r.branch() {
        Ok(branch) => print!("{}", branch.to_string()),
        Err(e) => panic!("failed to analyze branch: {}", e),
    }

    match r.status() {
        Ok(status) => print!("{}", status.to_string()),
        Err(e) => panic!("failed to get status: {}", e),
    }

    Ok(())
}

struct Repository {
    repository: git2::Repository,
}

impl Repository {
    fn status(&self) -> Result<RepoStatus, Error> {
        let mut options = git2::StatusOptions::new();
        options.include_untracked(true);
        options.show(git2::StatusShow::IndexAndWorkdir);
        let statuses = self.repository.statuses(Some(&mut options))?;

        let mut result = RepoStatus{
            new_files: None,
            modifications_staged: None,
            modifications: None,
            untracked: None,
            renames_staged: None,
            renames: None,
            deletions_staged: None,
            deletions: None,
        };

        for entry in statuses.iter() {
            match entry.status() {
                s if s.contains(git2::Status::INDEX_MODIFIED) => result.modifications_staged.replace(result.modifications_staged.unwrap_or(0) + 1),
                s if s.contains(git2::Status::WT_MODIFIED) => result.modifications.replace(result.modifications.unwrap_or(0) + 1),
                s if s.contains(git2::Status::INDEX_NEW) => result.new_files.replace(result.new_files.unwrap_or(0) + 1),
                s if s.contains(git2::Status::WT_NEW) => result.untracked.replace(result.untracked.unwrap_or(0) + 1),
                s if s.contains(git2::Status::INDEX_RENAMED) => result.renames_staged.replace(result.renames_staged.unwrap_or(0) + 1),
                s if s.contains(git2::Status::WT_RENAMED) => result.renames.replace(result.renames.unwrap_or(0) + 1),
                s if s.contains(git2::Status::INDEX_DELETED) => result.deletions_staged.replace(result.deletions_staged.unwrap_or(0) + 1),
                s if s.contains(git2::Status::WT_DELETED) => result.deletions.replace(result.deletions.unwrap_or(0) + 1),
                // s if s.contains(git2::Status::CONFLICTED) => match entry.head_to_index().unwrap().status() {

                // }
                _ => continue,
            };
        }

        Ok(result)
    }

    fn branch(&self) -> Result<BranchStatus, Error> {
        let head = match self.repository.head() {
            Ok(h) => h,
            Err(_e) => return Ok(BranchStatus {
                name: "detached".to_string(),
                local: None,
                upstream: None,
            }),
        };
        let mut name = head.name().unwrap();
        let branch_name = name;

        if name == "refs/heads/master" {
            name = "🅼"
        }

        let local = self
            .repository
            .find_branch("master", git2::BranchType::Local)
            .and_then(|master: git2::Branch| {
                self.repository
                    .graph_ahead_behind(head.target().unwrap(), master.get().target().unwrap())
            })
            .ok();

        let upstream = self
            .repository
            .branch_upstream_name(branch_name)
            .ok()
            .and_then(|bname_buf: git2::Buf| {
                bname_buf.as_str().map(|s| {s.to_string()})
            })
            .and_then( |bname| {
                self.repository
                    .graph_ahead_behind(head.target().unwrap(), self.repository.refname_to_id(&bname).unwrap())
                    .ok()
            });

        Ok(BranchStatus {
            name: name.to_string(),
            local,
            upstream,
        })
    }
}

struct RepoStatus {
    modifications_staged: Option<usize>,
    modifications: Option<usize>,
    new_files: Option<usize>,
    untracked: Option<usize>,
    renames_staged: Option<usize>,
    renames: Option<usize>,
    deletions_staged: Option<usize>,
    deletions: Option<usize>,
}

impl RepoStatus {
    fn to_string(&self) -> String {
        let mut result = String::new();

        result.push_str(&self.new_files.map(|i| { format!("{}{}", i, "N".green()) }).unwrap_or("".to_string()));
        result.push_str(&self.modifications_staged.map(|i| { format!("{}{}", i, "M".green()) }).unwrap_or("".to_string()));
        result.push_str(&self.renames_staged.map(|i| { format!("{}{}", i, "R".green()) }).unwrap_or("".to_string()));
        result.push_str(&self.deletions_staged.map(|i| { format!("{}{}", i, "D".green()) }).unwrap_or("".to_string()));
        result.push_str(&self.modifications.map(|i| { format!("{}{}", i, "M".red()) }).unwrap_or("".to_string()));
        result.push_str(&self.renames.map(|i| { format!("{}{}", i, "R".red()) }).unwrap_or("".to_string()));
        result.push_str(&self.deletions.map(|i| { format!("{}{}", i, "D".red()) }).unwrap_or("".to_string()));
        result.push_str(&self.untracked.map(|i| { format!("{}{}", i, "U".blue()) }).unwrap_or("".to_string()));

        result
    }
}

struct BranchStatus {
    name: String,
    local: Option<(usize, usize)>,
    upstream: Option<(usize, usize)>,
}

impl BranchStatus {
    fn upstream(&self) -> Option<String> {
        match self.upstream {
            Some((a, b)) if a > 0 && b > 0 => Some(format!("{}{}{}", "⇵".yellow(), a, b)),
            Some((a, 0)) if a > 0 => Some(format!("{}{}", "↓".red(), a)),
            Some((0, b)) if b > 0 => Some(format!("{}{}", "↑".green(), b)),
            Some((0, 0)) => Some("≡".to_string()),
            _ => Some("⚡".red().to_string()),
        }
    }

    fn local(&self) -> Option<String> {
        match self.local {
            Some((a, b)) if a > 0 && b > 0 => Some(format!("m{}{}{}", "↔".magenta(), a, b)),
            Some((a, 0)) if a > 0 => Some(format!("m{}{}", "←".magenta(), a)),
            Some((0, b)) if b > 0 => Some(format!("m{}{}", "→".magenta(), b)),
            _ => Some("⦰".red().to_string()),
        }
    }

    fn to_string(&self) -> String {
        let mut result = String::new();

        result.push_str(&self.name);
        result.push_str(&self.local().unwrap_or("".to_string()));
        result.push_str(&self.upstream().unwrap_or("".to_string()));

        result
    }
}
