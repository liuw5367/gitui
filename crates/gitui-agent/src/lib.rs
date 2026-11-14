//! Agent permission policy. The model never gets raw process or Keychain access.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Automation {
    Off,
    Commit,
    CommitAndPush,
}

impl Default for Automation {
    fn default() -> Self {
        Self::Off
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AgentPermissions {
    pub read: bool,
    pub stage: bool,
    pub commit: bool,
    pub push: bool,
    pub force_push: bool,
    pub discard: bool,
    pub protected_requires_approval: bool,
    pub automation: Automation,
}

impl Default for AgentPermissions {
    fn default() -> Self {
        Self {
            read: true,
            stage: true,
            commit: false,
            push: false,
            force_push: false,
            discard: false,
            protected_requires_approval: true,
            automation: Automation::Off,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentTool {
    Read,
    Stage,
    Commit,
    Push,
    ForcePush,
    Discard,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
}

impl AgentPermissions {
    pub fn decide(&self, tool: AgentTool) -> Decision {
        let allowed = match tool {
            AgentTool::Read => self.read,
            AgentTool::Stage => self.stage,
            AgentTool::Commit => self.commit,
            AgentTool::Push => self.push,
            AgentTool::ForcePush => self.force_push,
            AgentTool::Discard => self.discard,
        };
        if allowed {
            Decision::Allow
        } else {
            Decision::Deny
        }
    }

    pub fn max_automation(&self) -> Automation {
        match self.automation {
            Automation::CommitAndPush if !self.push => Automation::Commit,
            Automation::Commit if !self.commit => Automation::Off,
            automation => automation,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanStep {
    StageAll,
    Commit { message: String },
    Push,
}

impl PlanStep {
    pub fn tool(&self) -> AgentTool {
        match self {
            PlanStep::StageAll => AgentTool::Stage,
            PlanStep::Commit { .. } => AgentTool::Commit,
            PlanStep::Push => AgentTool::Push,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AgentPlan {
    pub steps: Vec<PlanStep>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanError {
    EmptyPlan,
    TooManySteps,
    EmptyCommitMessage,
    Denied(AgentTool),
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanError::EmptyPlan => write!(f, "plan has no steps"),
            PlanError::TooManySteps => write!(f, "plan has too many steps"),
            PlanError::EmptyCommitMessage => write!(f, "commit message is empty"),
            PlanError::Denied(tool) => write!(f, "permission denied for {tool:?}"),
        }
    }
}

impl AgentPlan {
    pub fn validate(&self, permissions: &AgentPermissions) -> Result<(), PlanError> {
        if self.steps.is_empty() {
            return Err(PlanError::EmptyPlan);
        }
        if self.steps.len() > 12 {
            return Err(PlanError::TooManySteps);
        }
        for step in &self.steps {
            if let PlanStep::Commit { message } = step {
                if message.trim().is_empty() {
                    return Err(PlanError::EmptyCommitMessage);
                }
            }
            if permissions.decide(step.tool()) == Decision::Deny {
                return Err(PlanError::Denied(step.tool()));
            }
        }
        Ok(())
    }
}

pub trait Planner {
    fn plan(&self, prompt: &str, commit_message: &str) -> Result<AgentPlan, PlanError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LocalPlanner;

impl Planner for LocalPlanner {
    fn plan(&self, prompt: &str, commit_message: &str) -> Result<AgentPlan, PlanError> {
        let mut steps = Vec::new();
        let mut has_commit = false;
        for raw in prompt.split_whitespace() {
            let token = raw
                .trim_matches(|ch: char| !ch.is_ascii_alphabetic())
                .to_ascii_lowercase();
            match token.as_str() {
                "stage" if !steps.iter().any(|step| matches!(step, PlanStep::StageAll)) => {
                    steps.push(PlanStep::StageAll);
                }
                "commit" if !has_commit => {
                    has_commit = true;
                    steps.push(PlanStep::Commit {
                        message: commit_message.trim().to_string(),
                    });
                }
                "push" if !steps.iter().any(|step| matches!(step, PlanStep::Push)) => {
                    steps.push(PlanStep::Push);
                }
                _ => {}
            }
        }
        if steps.is_empty() {
            Err(PlanError::EmptyPlan)
        } else {
            Ok(AgentPlan { steps })
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditEvent {
    pub summary: String,
    pub allowed: bool,
}

#[derive(Clone, Debug, Default)]
pub struct AuditLog {
    events: Vec<AuditEvent>,
}

impl AuditLog {
    pub fn append(&mut self, summary: impl Into<String>, allowed: bool) {
        self.events.push(AuditEvent {
            summary: summary.into(),
            allowed,
        });
    }

    pub fn events(&self) -> &[AuditEvent] {
        &self.events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_least_privilege() {
        let permissions = AgentPermissions::default();
        assert_eq!(permissions.decide(AgentTool::Read), Decision::Allow);
        assert_eq!(permissions.decide(AgentTool::Commit), Decision::Deny);
        assert_eq!(permissions.decide(AgentTool::Push), Decision::Deny);
        assert_eq!(permissions.decide(AgentTool::ForcePush), Decision::Deny);
        assert_eq!(permissions.decide(AgentTool::Discard), Decision::Deny);
    }

    #[test]
    fn enabled_tools_are_allowed() {
        let permissions = AgentPermissions {
            commit: true,
            push: true,
            force_push: true,
            discard: true,
            ..AgentPermissions::default()
        };
        for tool in [
            AgentTool::Read,
            AgentTool::Stage,
            AgentTool::Commit,
            AgentTool::Push,
            AgentTool::ForcePush,
            AgentTool::Discard,
        ] {
            assert_eq!(permissions.decide(tool), Decision::Allow);
        }
    }

    #[test]
    fn automation_cannot_exceed_permissions() {
        let permissions = AgentPermissions {
            automation: Automation::CommitAndPush,
            push: false,
            commit: true,
            ..AgentPermissions::default()
        };
        assert_eq!(permissions.max_automation(), Automation::Commit);
    }
    #[test]
    fn plan_requires_approval_and_permissions() {
        let permissions = AgentPermissions::default();
        let plan = AgentPlan {
            steps: vec![
                PlanStep::Commit {
                    message: "fix".to_string(),
                },
                PlanStep::Push,
            ],
        };
        assert_eq!(
            plan.validate(&permissions),
            Err(PlanError::Denied(AgentTool::Commit))
        );
    }

    #[test]
    fn plan_rejects_empty_commit_message() {
        let permissions = AgentPermissions {
            commit: true,
            ..AgentPermissions::default()
        };
        let plan = AgentPlan {
            steps: vec![PlanStep::Commit {
                message: "  ".to_string(),
            }],
        };
        assert_eq!(
            plan.validate(&permissions),
            Err(PlanError::EmptyCommitMessage)
        );
    }

    #[test]
    fn audit_log_is_append_only_from_api() {
        let mut log = AuditLog::default();
        log.append("commit staged", true);
        log.append("push origin/main", false);
        assert_eq!(log.events().len(), 2);
        assert!(!log.events()[1].allowed);
    }

    #[test]
    fn local_planner_builds_ordered_plan() {
        let plan = LocalPlanner
            .plan("stage commit push", "fix thing")
            .unwrap();
        assert_eq!(
            plan.steps,
            vec![
                PlanStep::StageAll,
                PlanStep::Commit {
                    message: "fix thing".to_string()
                },
                PlanStep::Push,
            ]
        );
    }

    #[test]
    fn local_planner_rejects_unknown_prompt() {
        assert_eq!(
            LocalPlanner.plan("summarize the repo", ""),
            Err(PlanError::EmptyPlan)
        );
    }

}
