use dioxus::prelude::*;
use crate::components::{AccountTile, AddAccountTile, PasswordPromptModal};
use crate::persistence;
use crate::state::UserStore;

/// Écran affiché tant que personne n'est connecté : une rangée de comptes
/// en cercles — cliquer dessus connecte directement, ou ouvre une demande
/// de mot de passe si le compte en a un — plus un cercle "+" pour créer
/// un nouveau compte.
#[component]
pub fn AuthScreen() -> Element {
    let mut user_store = use_context::<Signal<UserStore>>();
    let mut session = use_context::<Signal<Option<usize>>>();
    let mut password_prompt = use_signal(|| None::<(usize, String)>);

    rsx! {
        div { class: "auth-screen",
            div { class: "auth-box",
                h1 { "Connexion" }

                if user_store.read().users.is_empty() {
                    p { class: "auth-hint",
                        "Aucun compte pour l'instant, crée le premier ci-dessous."
                    }
                }

                div { class: "account-row",
                    for (user , user_id , username , needs_password) in user_store
                        .read()
                        .users
                        .iter()
                        .map(|u| (u.clone(), u.id, u.username.clone(), u.password_hash.is_some()))
                        .collect::<Vec<_>>()
                    {
                        AccountTile {
                            key: "{user_id}",
                            user,
                            onselect: move |_| {
                                if needs_password {
                                    password_prompt.set(Some((user_id, username.clone())));
                                } else {
                                    session.set(Some(user_id));
                                }
                            },
                            ondelete: move |_| {
                                user_store.write().remove_user(user_id);
                                persistence::delete_user_data(user_id);
                                if password_prompt().map(|(id, _)| id) == Some(user_id) {
                                    password_prompt.set(None);
                                }
                            },
                        }
                    }
                    AddAccountTile {}
                }
            }
        }

        if let Some((user_id, username)) = password_prompt() {
            PasswordPromptModal {
                user_id,
                username,
                onclose: move |_| password_prompt.set(None),
            }
        }
    }
}