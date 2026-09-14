mod model;
mod parse;

use asiba_core::{
    Availability, CollectContext, Module, ModuleError, ModuleId, Sample, Schedule, Snapshot,
    SudoMode, Transport,
};
use async_trait::async_trait;

pub use model::{GitInfo, Project, ProjectKind, ProjectsSnapshot, UnassignedProcess};

use crate::common::sections;

pub const ID: ModuleId = ModuleId("projects");
pub const KEY_COUNT: &str = "projects.count";

const SCAN_DIRS: &str = "/opt /srv /var/www /home/* /root /app /docker /data";
const MARKERS: &str = "-name .git -o -name 'docker-compose*.yml' -o -name 'docker-compose*.yaml' -o -name compose.yml -o -name compose.yaml -o -name package.json -o -name Cargo.toml -o -name pyproject.toml -o -name go.mod -o -name Dockerfile -o -name ecosystem.config.js";

pub struct ProjectsModule;

fn script() -> String {
    let markers = format!(
        "for d in {SCAN_DIRS}; do find \"$d\" -maxdepth 3 \\( {MARKERS} \\) -not -path '*/node_modules/*' -not -path '*/.git/*' -not -path '*/vendor/*' -not -path '*/target/*'; done"
    );
    let git = format!(
        "for d in {SCAN_DIRS}; do find \"$d\" -maxdepth 3 -name .git -type d; done | while read -r g; do r=\"${{g%/.git}}\"; printf '%s\\t%s\\t%s\\t%s\\n' \"$r\" \"$(git -C \"$r\" rev-parse --abbrev-ref HEAD)\" \"$(git -C \"$r\" log -1 --format='%h %ci %s')\" \"$(git -C \"$r\" status --porcelain | wc -l)\"; done"
    );
    let parts = [
        ("markers", markers.as_str()),
        ("git", git.as_str()),
        (
            "units",
            "systemctl show '*.service' -p Id -p WorkingDirectory -p MainPID -p ActiveState --no-pager",
        ),
        (
            "cwd",
            "for p in /proc/[0-9]*; do printf '%s\\t%s\\t%s\\n' \"${p#/proc/}\" \"$(readlink $p/cwd)\" \"$(cat $p/comm)\"; done",
        ),
        (
            "containers",
            "D=$(command -v docker || command -v podman); ids=$($D ps -aq); [ -n \"$ids\" ] && $D inspect --format '{{.Name}}\\t{{index .Config.Labels \"com.docker.compose.project\"}}\\t{{index .Config.Labels \"com.docker.compose.project.working_dir\"}}\\t{{.State.Status}}' $ids",
        ),
        ("listen", "ss -tlnpH"),
    ];
    sections::script(&parts)
}

#[async_trait]
impl Module for ProjectsModule {
    fn id(&self) -> ModuleId {
        ID
    }

    fn title(&self) -> &'static str {
        "Проекты"
    }

    fn schedule(&self) -> Schedule {
        Schedule::Slow
    }

    async fn detect(&self, _transport: &dyn Transport) -> Result<Availability, ModuleError> {
        Ok(Availability::Available)
    }

    async fn collect(
        &self,
        transport: &dyn Transport,
        _context: &CollectContext,
    ) -> Result<Snapshot, ModuleError> {
        let command = script();
        let output = match transport.sudo_mode() {
            SudoMode::None => transport.exec(&command).await?,
            _ => match transport.exec_root(&command).await {
                Ok(output) if output.is_success() => output,
                _ => transport.exec(&command).await?,
            },
        };
        let snapshot = parse::projects_snapshot(&output.stdout)?;
        let samples = vec![Sample::new(KEY_COUNT, snapshot.projects.len() as f64)];
        Ok(Snapshot::new(snapshot).with_samples(samples))
    }
}
