#!/usr/bin/env bash
# 生成确定性测试仓库与对应的 bare 远程。
#
#   ./fixtures/gen-repo.sh /tmp/gitui-fixture
#
# 产出：
#   <目标>/                工作仓库，三分支、标签、贮藏、子模块、冲突提交、钩子
#   <目标>-remote.git/      本机 bare 远程，供 fetch/pull/push 验收
#
# 同一输入每次生成同样的历史（提交时间固定），便于截图与断言比较。
set -euo pipefail

target="${1:-/tmp/gitui-fixture}"
remote="${target}-remote.git"

if [ -e "$target" ] || [ -e "$remote" ]; then
  echo "目标已存在，先删除：$target / $remote" >&2
  exit 1
fi

export GIT_AUTHOR_DATE="2026-11-01T09:00:00+0000"
export GIT_COMMITTER_DATE="2026-11-01T09:00:00+0000"
export GIT_AUTHOR_NAME="Mia Chen"
export GIT_AUTHOR_EMAIL="mia@example.com"
export GIT_COMMITTER_NAME="Mia Chen"
export GIT_COMMITTER_EMAIL="mia@example.com"

git init -q -b main "$target"
cd "$target"
git config user.name "Mia Chen"
git config user.email "mia@example.com"
git config commit.gpgsign false

commit() {
  # commit <message> [author]
  local message="$1"
  local author="${2:-Mia Chen}"
  if [ "$author" != "Mia Chen" ]; then
    GIT_AUTHOR_NAME="$author" GIT_AUTHOR_EMAIL="author@example.com" git commit -q -m "$message"
  else
    git commit -q -m "$message"
  fi
  GIT_AUTHOR_DATE="$(date -u -v+1M +%Y-%m-%dT%H:%M:%S+0000 2>/dev/null || date -u +%Y-%m-%dT%H:%M:%S+0000)"
  GIT_COMMITTER_DATE="$GIT_AUTHOR_DATE"
}

mkdir -p src
cat > src/main.rs <<'EOF'
fn main() {
    println!("hello");
}
EOF
git add -A
commit "Initial commit"

cat > src/diff.rs <<'EOF'
pub fn render(patch: &str) -> String {
    patch.to_string()
}
EOF
git add -A
commit "Add diff renderer"

cat > src/diff.rs <<'EOF'
pub fn render(patch: &str) -> String {
    let mut out = String::new();
    for line in patch.lines() {
        out.push_str(line);
        out.push('\n');
    }
    out
}
EOF
git add -A
commit "Wrap long diff lines"

git checkout -q -b feature/diff-view
cat > src/highlight.rs <<'EOF'
pub fn highlight(words: &[&str]) -> usize {
    words.len()
}
EOF
git add -A
commit "Add inline word highlight"

git checkout -q main
git merge -q --no-ff feature/diff-view -m "Merge feature/diff-view"
commit "Merge feature/diff-view" || true

git checkout -q -b feature/agent
cat > src/agent.rs <<'EOF'
pub fn plan() -> Vec<&'static str> {
    vec!["stage", "commit"]
}
EOF
git add -A
commit "Add agent planner stub" "Jonas Weber"
git checkout -q main

git tag v1.0.0 HEAD~3
git tag v1.1.0 HEAD

# 大 diff：8k 行，供滚动性能与渲染验证。
python3 - "$target" <<'PY'
import sys
root = sys.argv[1]
lines = [f"line {index}" for index in range(1, 4001)]
open(f"{root}/src/large.txt", "w").write("\n".join(lines) + "\n")
PY
git add -A
commit "Add generated large file"

python3 - "$target" <<'PY'
import sys
root = sys.argv[1]
lines = [f"line {index}" if index % 97 else f"changed {index}" for index in range(1, 8001)]
open(f"{root}/src/large.txt", "w").write("\n".join(lines) + "\n")
PY
git add -A
commit "Regenerate large file with 8k lines"

# 子模块：加一个本地占位仓库。
submodule_source="$(mktemp -d)"
git -C "$submodule_source" init -q -b main
cat > "$submodule_source/README.md" <<'EOF'
submodule fixture
EOF
git -C "$submodule_source" add -A
git -C "$submodule_source" -c user.name=Fixture -c user.email=fixture@example.com commit -q -m "submodule init"
git -c protocol.file.allow=always submodule add -q "$submodule_source" vendor/submodule
git add -A
commit "Add vendor submodule"

# 钩子：pre-commit 在文件里带 "hook-fail" 时失败，用于提交失败路径。
mkdir -p .git/hooks
cat > .git/hooks/pre-commit <<'EOF'
#!/bin/sh
if grep -q "hook-fail" .git/hook-flag 2>/dev/null; then
  echo "pre-commit: 检测到 hook-fail" >&2
  exit 1
fi
echo "pre-commit: ok"
EOF
chmod +x .git/hooks/pre-commit

# 工作区改动：一个已暂存、一个未暂存、一个未跟踪。
cat > src/staged.txt <<'EOF'
staged change
EOF
git add src/staged.txt
printf 'unstaged change\n' >> src/main.rs
printf 'untracked\n' > src/untracked.txt

# 贮藏：先提交一次再贮藏，保证列表里有一条。
git stash push -q -u -m "WIP: composer states"
git stash pop -q

# 冲突场景在独立分支准备，供合并验收使用。
git checkout -q -b conflict/side HEAD~4
cat > src/conflict.txt <<'EOF'
side version
EOF
git add -A
commit "Side change for conflict"
git checkout -q main
cat > src/conflict.txt <<'EOF'
main version
EOF
git add -A
commit "Main change for conflict"

# bare 远程：推送 main 与标签。
git init -q --bare "$remote"
git remote add origin "$remote"
git push -q origin main --tags
git branch -q --set-upstream-to=origin/main main

echo "已生成："
echo "  仓库：$target"
echo "  远程：$remote"
echo "  分支：$(git branch --format='%(refname:short)' | tr '\n' ' ')"
echo "  提交：$(git rev-list --count HEAD)"
