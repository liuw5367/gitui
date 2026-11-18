//! 根据提交 SHA 与 parents 为历史列表计算 DAG 泳道。

use domain::Commit;
use std::collections::HashSet;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct GraphRow {
    pub(crate) node_lane: usize,
    /// 当前行需要连续绘制的竖向泳道。
    pub(crate) active_lanes: Vec<usize>,
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
        lanes[node_lane] = Some(commit.sha.clone());
        let mut active = lanes
            .iter()
            .enumerate()
            .filter_map(|(index, sha)| sha.as_ref().map(|_| index))
            .collect::<Vec<_>>();

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
        active.extend(
            lanes
                .iter()
                .enumerate()
                .filter_map(|(index, sha)| sha.as_ref().map(|_| index)),
        );
        active.push(node_lane);
        active.sort_unstable();
        active.dedup();
        let lane_count = active
            .last()
            .map(|lane| lane + 1)
            .unwrap_or(node_lane + 1)
            .max(lanes.len());
        rows.push(GraphRow {
            node_lane,
            active_lanes: active,
            parent_lanes,
            lane_count,
        });
    }

    return rows;
}
