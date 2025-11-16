//! Production seed - baseline data for new deployments
//!
//! Seeds system defaults for models, agents, and sample axiology tags

use crate::database::Database;
use crate::Result;
use tracing::info;

/// Seed system default models (user_id = NULL)
pub async fn seed_default_models(db: &Database) -> Result<usize> {
    let models = vec![
        ("openai/gpt-oss-120b", "GPT OSS 120B", "OpenAI", 1),
        ("openai/gpt-oss-20b", "GPT OSS 20B", "OpenAI", 2),
        ("anthropic/claude-sonnet-4.5", "Claude Sonnet 4.5", "Anthropic", 3),
        ("anthropic/claude-opus-4.1", "Claude Opus 4.1", "Anthropic", 4),
        ("anthropic/claude-haiku-4.5", "Claude Haiku 4.5", "Anthropic", 5),
        ("openai/gpt-5", "GPT-5", "OpenAI", 6),
        ("google/gemini-2.5-pro", "Gemini 2.5 Pro", "Google", 7),
        ("google/gemini-2.5-flash", "Gemini 2.5 Flash", "Google", 8),
        ("xai/grok-4", "Grok 4", "xAI", 9),
        ("moonshotai/kimi-k2-thinking", "Kimi K2 Thinking", "Moonshot AI", 10),
    ];

    let count = models.len();
    for (model_id, display_name, provider, sort_order) in &models {
        sqlx::query!(
            r#"
            INSERT INTO app.models (user_id, model_id, display_name, provider, enabled, sort_order)
            VALUES (NULL, $1, $2, $3, true, $4)
            ON CONFLICT (model_id) WHERE user_id IS NULL DO NOTHING
            "#,
            model_id,
            display_name,
            provider,
            sort_order
        )
        .execute(db.pool())
        .await?;
    }

    info!("✅ Seeded {} system default models", count);
    Ok(count)
}

/// Seed system default agents (user_id = NULL)
pub async fn seed_default_agents(db: &Database) -> Result<usize> {
    let agents = vec![
        (
            "analytics",
            "Analytics",
            "Specializes in data exploration, location analysis, and visualizations",
            "#3b82f6",
            "ri:bar-chart-line",
            1,
        ),
        (
            "research",
            "Research",
            "Focuses on narratives, semantic search, values, and connecting ideas",
            "#8b5cf6",
            "ri:book-open-line",
            2,
        ),
        (
            "general",
            "General",
            "Adaptive assistant for general queries and mixed tasks",
            "#6b7280",
            "ri:chat-3-line",
            3,
        ),
    ];

    let count = agents.len();
    for (agent_id, name, description, color, icon, sort_order) in &agents {
        sqlx::query!(
            r#"
            INSERT INTO app.agents (user_id, agent_id, name, description, color, icon, enabled, sort_order)
            VALUES (NULL, $1, $2, $3, $4, $5, true, $6)
            ON CONFLICT (agent_id) WHERE user_id IS NULL DO NOTHING
            "#,
            agent_id,
            name,
            description,
            color,
            icon,
            sort_order
        )
        .execute(db.pool())
        .await?;
    }

    info!("✅ Seeded {} system default agents", count);
    Ok(count)
}

/// Seed sample axiology tags via tasks
/// Creates sample tasks with common tags to populate autocomplete
pub async fn seed_axiology_tags(db: &Database) -> Result<usize> {
    let sample_tasks = vec![
        ("Work projects", Some("Professional and career-related tasks"), vec!["work".to_string()]),
        ("Family time", Some("Spending quality time with loved ones"), vec!["family".to_string(), "relational".to_string()]),
        ("Exercise routine", Some("Physical fitness and wellbeing"), vec!["health".to_string()]),
        ("Personal development", Some("Self-improvement and growth"), vec!["personal".to_string()]),
        ("Creative pursuits", Some("Artistic and creative activities"), vec!["creative".to_string()]),
        ("Spiritual practice", Some("Meditation, reflection, and inner work"), vec!["spiritual".to_string()]),
        ("New experiences", Some("Trying new things and adventures"), vec!["experiential".to_string()]),
        ("Building relationships", Some("Deepening connections with others"), vec!["relational".to_string()]),
    ];

    let count = sample_tasks.len();
    for (title, description, tags) in &sample_tasks {
        sqlx::query!(
            r#"
            INSERT INTO elt.axiology_task (title, description, tags)
            VALUES ($1, $2, $3)
            "#,
            title,
            description.as_deref(),
            tags as &[String]
        )
        .execute(db.pool())
        .await?;
    }

    info!("✅ Seeded {} sample axiology tasks with tags", count);
    Ok(count)
}

/// Seed default tools metadata
pub async fn seed_default_tools(db: &Database) -> Result<usize> {
    let tools = vec![
        (
            "queryLocationMap",
            "Location Map",
            "Visualize locations and places you've visited on an interactive map",
            "analytics",
            "mdi:map-marker",
            true,
            1,
        ),
        (
            "queryPursuits",
            "Tasks & Goals",
            "View and manage your tasks, initiatives, and life pursuits",
            "analytics",
            "mdi:target",
            true,
            2,
        ),
    ];

    let count = tools.len();
    for (id, name, description, category, icon, is_pinnable, display_order) in &tools {
        sqlx::query!(
            r#"
            INSERT INTO app.tools (id, name, description, category, icon, is_pinnable, display_order)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                description = EXCLUDED.description,
                category = EXCLUDED.category,
                icon = EXCLUDED.icon,
                is_pinnable = EXCLUDED.is_pinnable,
                display_order = EXCLUDED.display_order,
                updated_at = NOW()
            "#,
            id,
            name,
            description,
            category,
            icon,
            is_pinnable,
            display_order
        )
        .execute(db.pool())
        .await?;
    }

    info!("✅ Seeded {} default tools", count);
    Ok(count)
}

/// Seed all production defaults
pub async fn seed_production_data(db: &Database) -> Result<()> {
    info!("🌱 Seeding production defaults...");

    seed_default_models(db).await?;
    seed_default_agents(db).await?;
    seed_default_tools(db).await?;
    seed_axiology_tags(db).await?;

    info!("✅ Production seeding completed successfully");
    Ok(())
}
