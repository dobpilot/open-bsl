use super::{InfoBaseUser, principal_candidates};
use open_sdbl::metadata::UserRow;

#[test]
fn acquired_full_names_match_authenticated_native_sessions() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/native-user-full-names");
    for name in ["LangRussian", "LangEnglish", "LangDefault"] {
        let native = std::fs::read_to_string(root.join(format!("{name}.platform.txt"))).unwrap();
        let native_name = native
            .lines()
            .find_map(|line| line.strip_prefix("USER="))
            .unwrap();
        let full_name = native
            .lines()
            .find_map(|line| line.strip_prefix("FULLNAME="))
            .unwrap();
        assert_ne!(native_name, full_name);
        let bytes = std::fs::read(root.join(format!("{name}.data"))).unwrap();
        let mut user = InfoBaseUser::new(
            UserRow {
                name: native_name,
                description: full_name,
                os_name: "",
                email: "",
                show_in_list: false,
                standard_authentication: true,
                administrative: false,
            },
            &bytes,
        )
        .unwrap();
        let candidates = principal_candidates(std::slice::from_ref(&user));
        let principal = candidates[0].principal();
        assert_eq!(principal.name(), native_name);
        assert_eq!(principal.full_name(), full_name);
        // Уже созданный principal не перечитывает изменившуюся запись.
        user.data.full_name = "Изменённое имя".into();
        assert_eq!(principal.full_name(), full_name);
    }
}
