use std::collections::HashMap;

// 1. Définissez vos templates métiers et leurs données obligatoires
#[derive(Debug, Clone)]
pub enum AppTemplate {
    // Welcome {
    //     user_name: String,
    //     activation_link: String,
    // },
    FirstConnectPasswordReset {
        target_name: String,
        mairie_name: String,
        reset_link: String,
        expires_time: u32,
    },
}

impl AppTemplate {
    // Retourne l'alias textuel visible sur le dashboard Resend
    pub fn template_alias(&self) -> &'static str {
        match self {
            // AppTemplate::Welcome { .. } => "welcome-email", // Votre alias Resend
            AppTemplate::FirstConnectPasswordReset { .. } => "first-connect", // Votre alias Resend
        }
    }

    // Convertit les variables structurées en HashMap pour Resend
    pub fn into_variables(self) -> HashMap<String, serde_json::Value> {
        let mut vars = HashMap::new();
        match self {
            // AppTemplate::Welcome {
            //     user_name,
            //     activation_link,
            // } => {
            //     vars.insert("user_name".to_string(), serde_json::json!(user_name));
            //     vars.insert(
            //         "activation_link".to_string(),
            //         serde_json::json!(activation_link),
            //     );
            // }
            AppTemplate::FirstConnectPasswordReset {
                target_name,
                mairie_name,
                reset_link,
                expires_time,
            } => {
                vars.insert("target_name".to_string(), serde_json::json!(target_name));
                vars.insert("mairie_name".to_string(), serde_json::json!(mairie_name));
                vars.insert("reset_link".to_string(), serde_json::json!(reset_link));
                vars.insert(
                    "expires_in_minutes".to_string(),
                    serde_json::json!(expires_time),
                );
            }
        }
        vars
    }
}
