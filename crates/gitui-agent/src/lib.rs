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
}
