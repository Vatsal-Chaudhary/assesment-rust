use sqlx::PgPool;
use uuid::Uuid;
use crate::models::{Task, TaskPriority, TaskStatus, TaskDto};

pub struct TaskRepository;

impl TaskRepository {
    pub async fn create_task(
        pool: &PgPool,
        title: &str,
        description: Option<String>,
        priority: TaskPriority,
        created_by: Uuid,
    ) -> Result<Task, sqlx::Error> {
        sqlx::query_as::<_, Task>(
            "INSERT INTO tasks (title, description, priority, created_by, status) 
             VALUES ($1, $2, $3, $4, 'todo') 
             RETURNING id, title, description, status, priority, created_by, assigned_to_id, created_at"
        )
        .bind(title)
        .bind(description)
        .bind(priority)
        .bind(created_by)
        .fetch_one(pool)
        .await
    }

    // Assigns an arbitrary batch of tasks to a user inside an atomic transaction block
    pub async fn assign_tasks_to_user(
        pool: &PgPool,
        user_id: Uuid,
        task_ids: &[Uuid],
    ) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;

        for task_id in task_ids {
            sqlx::query("UPDATE tasks SET assigned_to_id = $1 WHERE id = $2")
                .bind(user_id)
                .bind(task_id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    // Fetches the exact view with relational strings requested by GET /tasks/view-my-tasks
    pub async fn get_tasks_for_user(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Vec<TaskDto>, sqlx::Error> {
        let tasks = sqlx::query_as::<_, TaskDto>(
            "SELECT t.id, t.title, t.status, t.priority, u.email as assigned_to 
             FROM tasks t
             INNER JOIN users u ON t.assigned_to_id = u.id
             WHERE t.assigned_to_id = $1"
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        Ok(tasks)
    }
}
