use argon2::{
    password_hash::{phc::PasswordHash, PasswordHasher, PasswordVerifier},
    Argon2,
};

/// Hache un mot de passe en clair avec Argon2 (le sel est généré
/// automatiquement en interne) et retourne la chaîne résultante (format PHC),
/// prête à être stockée. Ne conserve jamais le mot de passe original.
pub fn hash_password(password: &str) -> String {
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes())
        .expect("le hachage ne devrait pas échouer")
        .to_string()
}

/// Vérifie qu'un mot de passe en clair correspond à un hash PHC donné.
/// Retourne `false` si le hash est mal formé ou si le mot de passe ne correspond pas.
pub fn verify_password(password: &str, hash: &str) -> bool {
    let Ok(parsed_hash) = PasswordHash::new(hash) else {
        return false;
    };

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}