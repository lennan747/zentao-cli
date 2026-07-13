/// 禅道专业版 9.0.3 旧版 JSON 路由。
pub struct Routes;

impl Routes {
    pub fn login(server: &str) -> String {
        format!("{}/user-login.html", server.trim_end_matches('/'))
    }

    pub fn refresh_random(server: &str) -> String {
        format!("{}/user-refreshRandom.html", server.trim_end_matches('/'))
    }

    pub fn project_index(server: &str) -> String {
        format!("{}/project-index.json", server.trim_end_matches('/'))
    }

    pub fn project_view(server: &str, id: &str) -> String {
        format!("{}/project-view-{}.json", server.trim_end_matches('/'), id)
    }

    pub fn task_list(server: &str) -> String {
        format!("{}/my-task.json", server.trim_end_matches('/'))
    }

    pub fn task_view(server: &str, id: &str) -> String {
        format!("{}/task-view-{}.json", server.trim_end_matches('/'), id)
    }

    pub fn bug_list_assigned_to(server: &str) -> String {
        format!("{}/my-bug-assignedTo.json", server.trim_end_matches('/'))
    }

    pub fn bug_view(server: &str, id: &str) -> String {
        format!("{}/bug-view-{}.json", server.trim_end_matches('/'), id)
    }
}
