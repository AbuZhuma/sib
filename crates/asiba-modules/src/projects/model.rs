#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProjectKind {
    Git,
    Compose,
    Dockerfile,
    Node,
    Rust,
    Python,
    Go,
    Pm2,
}

impl ProjectKind {
    pub fn from_marker(file_name: &str) -> Option<Self> {
        Some(match file_name {
            ".git" => Self::Git,
            "compose.yml" | "compose.yaml" => Self::Compose,
            name if name.starts_with("docker-compose") => Self::Compose,
            "Dockerfile" => Self::Dockerfile,
            "package.json" => Self::Node,
            "Cargo.toml" => Self::Rust,
            "pyproject.toml" => Self::Python,
            "go.mod" => Self::Go,
            "ecosystem.config.js" => Self::Pm2,
            _ => return None,
        })
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Git => "git",
            Self::Compose => "compose",
            Self::Dockerfile => "dockerfile",
            Self::Node => "node",
            Self::Rust => "rust",
            Self::Python => "python",
            Self::Go => "go",
            Self::Pm2 => "pm2",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitInfo {
    pub branch: String,
    pub last_commit: String,
    pub dirty_files: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub name: String,
    pub path: String,
    pub kinds: Vec<ProjectKind>,
    pub git: Option<GitInfo>,
    pub units: Vec<String>,
    pub containers: Vec<String>,
    pub processes: Vec<(u32, String)>,
    pub ports: Vec<u16>,
}

impl Project {
    pub fn kinds_label(&self) -> String {
        self.kinds
            .iter()
            .map(|k| k.label())
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn is_active(&self) -> bool {
        !self.units.is_empty() || !self.containers.is_empty() || !self.processes.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnassignedProcess {
    pub pid: u32,
    pub comm: String,
    pub cwd: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectsSnapshot {
    pub projects: Vec<Project>,
    pub unassigned: Vec<UnassignedProcess>,
}

impl ProjectsSnapshot {
    pub fn active(&self) -> impl Iterator<Item = &Project> {
        self.projects.iter().filter(|p| p.is_active())
    }
}
