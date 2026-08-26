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

    // ---- 写操作路由（表单 POST，`.json` 后缀返回包络）----

    pub fn task_create(server: &str, project: &str) -> String {
        format!("{}/task-create-{}.json", base(server), project)
    }

    pub fn task_edit(server: &str, id: &str) -> String {
        format!("{}/task-edit-{}.json", base(server), id)
    }

    pub fn task_start(server: &str, id: &str) -> String {
        format!("{}/task-start-{}.json", base(server), id)
    }

    pub fn task_finish(server: &str, id: &str) -> String {
        format!("{}/task-finish-{}.json", base(server), id)
    }

    pub fn task_cancel(server: &str, id: &str) -> String {
        format!("{}/task-cancel-{}.json", base(server), id)
    }

    pub fn task_close(server: &str, id: &str) -> String {
        format!("{}/task-close-{}.json", base(server), id)
    }

    pub fn task_activate(server: &str, id: &str) -> String {
        format!("{}/task-activate-{}.json", base(server), id)
    }

    pub fn task_comment(server: &str, id: &str) -> String {
        format!("{}/action-comment-task-{}.html", base(server), id)
    }

    /// Bug 创建；旧版 9.0.3 路由为 product-branch-module，branch/module 默认 0。
    pub fn bug_create(server: &str, product: &str) -> String {
        format!("{}/bug-create-{}-0-0.json", base(server), product)
    }

    pub fn bug_edit(server: &str, id: &str) -> String {
        format!("{}/bug-edit-{}.json", base(server), id)
    }

    pub fn bug_resolve(server: &str, id: &str) -> String {
        format!("{}/bug-resolve-{}.json", base(server), id)
    }

    pub fn bug_activate(server: &str, id: &str) -> String {
        format!("{}/bug-activate-{}.json", base(server), id)
    }

    pub fn bug_close(server: &str, id: &str) -> String {
        format!("{}/bug-close-{}.json", base(server), id)
    }

    pub fn bug_confirm(server: &str, id: &str) -> String {
        format!("{}/bug-confirm-{}.json", base(server), id)
    }

    pub fn bug_comment(server: &str, id: &str) -> String {
        format!("{}/action-comment-bug-{}.html", base(server), id)
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

    #[test]
    fn write_routes_use_json_suffix() {
        assert_eq!(
            Routes::task_edit("https://x.com", "946"),
            "https://x.com/task-edit-946.json"
        );
        assert_eq!(
            Routes::bug_create("https://x.com", "10"),
            "https://x.com/bug-create-10-0-0.json"
        );
        assert_eq!(
            Routes::task_comment("https://x.com", "946"),
            "https://x.com/action-comment-task-946.html"
        );
    }
}
