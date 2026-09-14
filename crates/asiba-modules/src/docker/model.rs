#[derive(Debug, Clone, PartialEq)]
pub struct ContainerStats {
    pub cpu_pct: f64,
    pub mem_usage: u64,
    pub mem_limit: u64,
    pub net_rx: u64,
    pub net_tx: u64,
    pub block_read: u64,
    pub block_write: u64,
    pub pids: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Container {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: String,
    pub status: String,
    pub ports: String,
    pub restart_count: u32,
    pub health: Option<String>,
    pub restart_policy: String,
    pub exit_code: i32,
    pub started_at: String,
    pub compose_project: Option<String>,
    pub compose_service: Option<String>,
    pub compose_dir: Option<String>,
    pub stats: Option<ContainerStats>,
}

impl Container {
    pub fn is_running(&self) -> bool {
        self.state == "running"
    }

    pub fn is_unhealthy(&self) -> bool {
        self.health.as_deref() == Some("unhealthy")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub id: String,
    pub repository: String,
    pub tag: String,
    pub size_bytes: u64,
    pub created: String,
}

impl Image {
    pub fn is_dangling(&self) -> bool {
        self.repository == "<none>" || self.tag == "<none>"
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DockerSnapshot {
    pub version: String,
    pub containers: Vec<Container>,
    pub images: Vec<Image>,
    pub volumes: u32,
    pub networks: Vec<String>,
}

impl DockerSnapshot {
    pub fn running_count(&self) -> usize {
        self.containers.iter().filter(|c| c.is_running()).count()
    }

    pub fn stopped_count(&self) -> usize {
        self.containers.len() - self.running_count()
    }

    pub fn compose_projects(&self) -> Vec<String> {
        let mut projects: Vec<String> = self
            .containers
            .iter()
            .filter_map(|c| c.compose_project.clone())
            .collect();
        projects.sort();
        projects.dedup();
        projects
    }

    pub fn images_size(&self) -> u64 {
        self.images.iter().map(|i| i.size_bytes).sum()
    }
}
