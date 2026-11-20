//! 根据提交 SHA 与 parents 为历史列表计算 DAG 泳道。

use domain::Commit;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct GraphRow {
    pub(crate) node_lane: usize,
    /// 从行顶端连到当前节点的泳道。
    pub(crate) top_lanes: Vec<usize>,
    /// 从当前节点向行底端延续的泳道。
    pub(crate) bottom_lanes: Vec<usize>,
    /// 从当前提交连接到额外 parent 泳道的横向线段。
    pub(crate) parent_lanes: Vec<usize>,
    pub(crate) lane_count: usize,
}

pub(crate) fn layout(commits: &[Commit]) -> Vec<GraphRow> {
    let known = commits
        .iter()
        .map(|commit| commit.sha.as_str())
        .collect::<HashSet<_>>();
    let mut lanes: Vec<Option<String>> = Vec::new();
    let mut rows = Vec::with_capacity(commits.len());

    for commit in commits {
        let node_lane = lanes
            .iter()
            .position(|lane| lane.as_deref() == Some(commit.sha.as_str()))
            .or_else(|| lanes.iter().position(Option::is_none))
            .unwrap_or(lanes.len());
        if node_lane == lanes.len() {
            lanes.push(None);
        }
        let node_has_incoming = lanes[node_lane].as_deref() == Some(commit.sha.as_str());
        lanes[node_lane] = Some(commit.sha.clone());
        let mut top_lanes = lanes
            .iter()
            .enumerate()
            .filter_map(|(index, sha)| sha.as_ref().map(|_| index))
            .collect::<Vec<_>>();
        if !node_has_incoming {
            top_lanes.retain(|lane| *lane != node_lane);
        }

        lanes[node_lane] = None;
        let mut parent_lanes = Vec::new();
        let mut has_main_parent = false;
        for parent in commit
            .parents
            .iter()
            .filter(|parent| known.contains(parent.as_str()))
        {
            let is_main_parent = !has_main_parent;
            has_main_parent = true;
            let parent_lane = if let Some(index) = lanes
                .iter()
                .position(|lane| lane.as_deref() == Some(parent.as_str()))
            {
                index
            } else {
                let index = if is_main_parent {
                    node_lane
                } else {
                    lanes
                        .iter()
                        .enumerate()
                        .skip(node_lane + 1)
                        .find_map(|(index, lane)| lane.is_none().then_some(index))
                        .or_else(|| lanes.iter().position(Option::is_none))
                        .unwrap_or(lanes.len())
                };
                if index == lanes.len() {
                    lanes.push(None);
                }
                lanes[index] = Some(parent.clone());
                index
            };
            if parent_lane != node_lane && !parent_lanes.contains(&parent_lane) {
                parent_lanes.push(parent_lane);
            }
        }

        while lanes.last().is_some_and(Option::is_none) {
            lanes.pop();
        }
        let bottom_lanes = lanes
            .iter()
            .enumerate()
            .filter_map(|(index, sha)| sha.as_ref().map(|_| index))
            .collect::<Vec<_>>();
        let mut occupied = top_lanes.clone();
        occupied.extend(
            lanes
                .iter()
                .enumerate()
                .filter_map(|(index, sha)| sha.as_ref().map(|_| index)),
        );
        occupied.push(node_lane);
        occupied.sort_unstable();
        occupied.dedup();
        let lane_count = occupied
            .last()
            .map(|lane| lane + 1)
            .unwrap_or(node_lane + 1)
            .max(lanes.len());
        rows.push(GraphRow {
            node_lane,
            top_lanes,
            bottom_lanes,
            parent_lanes,
            lane_count,
        });
    }

    return rows;
}

/// Reconnect visible commits through ancestors omitted by date, author, path, message or quick
/// search filters. Each visible parent is connected to the nearest visible ancestors on that path.
pub(crate) fn layout_filtered(visible: &[Commit], ancestors: &[Commit]) -> Vec<GraphRow> {
    if visible.is_empty() {
        return Vec::new();
    }
    let visible_shas = visible
        .iter()
        .map(|commit| commit.sha.as_str())
        .collect::<HashSet<_>>();
    let ancestors_by_sha = ancestors
        .iter()
        .map(|commit| (commit.sha.as_str(), commit))
        .collect::<HashMap<_, _>>();
    let projected = visible
        .iter()
        .map(|commit| {
            let mut projected = commit.clone();
            let mut parents = Vec::new();
            let mut visited = HashSet::new();
            for parent in &commit.parents {
                connect_visible_ancestors(
                    parent,
                    &visible_shas,
                    &ancestors_by_sha,
                    &mut visited,
                    &mut parents,
                );
            }
            projected.parents = parents;
            projected
        })
        .collect::<Vec<_>>();
    return layout(&projected);
}

fn connect_visible_ancestors(
    sha: &str,
    visible_shas: &HashSet<&str>,
    ancestors_by_sha: &HashMap<&str, &Commit>,
    visited: &mut HashSet<String>,
    output: &mut Vec<String>,
) {
    if visible_shas.contains(sha) {
        if !output.iter().any(|ancestor| ancestor == sha) {
            output.push(sha.to_string());
        }
        return;
    }
    if !visited.insert(sha.to_string()) {
        return;
    }
    let Some(commit) = ancestors_by_sha.get(sha) else {
        return;
    };
    for parent in &commit.parents {
        connect_visible_ancestors(parent, visible_shas, ancestors_by_sha, visited, output);
    }
}
