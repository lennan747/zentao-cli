/// 禅道专业版 9.0.3 旧版 JSON 路由。
pub struct Routes;

fn base(server: &str) -> String {
    server.trim_end_matches('/').to_string()
}

impl Routes {
    /// 登录页。`Lw==` 是 `/` 的 Base64；访问它会建立绑定 verifyRand 的 zentaosid 会话。
    pub fn login_page(server: &str) -> String {
        format!("{}/user-login-Lw==.html", base(server))
    }

    pub fn login(server: &str) -> String {
        format!("{}/user-login.html", base(server))
    }

    pub fn refresh_random(server: &str) -> String {
        format!("{}/user-refreshRandom.html", base(server))
    }

    pub fn project_index(server: &str) -> String {
        format!("{}/project-index.json", base(server))
    }

    /// 按状态筛选项目列表；`status=0` 表示全部。
    pub fn project_all(server: &str, status: &str) -> String {
        format!("{}/project-all-{}.json", base(server), status)
    }

    pub fn project_view(server: &str, id: &str) -> String {
        format!("{}/project-view-{}.json", base(server), id)
    }

    pub fn my_task(server: &str) -> String {
        format!("{}/my-task.json", base(server))
    }

    pub fn task_view(server: &str, id: &str) -> String {
        format!("{}/task-view-{}.json", base(server), id)
    }

    pub fn my_bug_assigned_to(server: &str) -> String {
        format!("{}/my-bug-assignedTo.json", base(server))
    }

    pub fn bug_view(server: &str, id: &str) -> String {
        format!("{}/bug-view-{}.json", base(server), id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_trim_trailing_slash() {
        assert_eq!(
            Routes::my_task("https://x.com/"),
            "https://x.com/my-task.json"
        );
    }
}
