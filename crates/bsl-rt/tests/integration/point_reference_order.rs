use bsl_rt::uuid::to_1c_bytes;
use std::cmp::Ordering;

#[test]
fn physical_reference_order_matches_native_point_in_time_group_precedence() {
    // Каждая ссылка содержит единицу только в одной группе GUID.
    // Ожидаемый порядок взят из Compare реальной платформы, а не из
    // перестановки байтов, которую проверяет этот тест.
    let positions = [3, 5, 7, 9, 15];
    let mut actual = Vec::new();
    for (first_group, first_position) in positions.iter().enumerate() {
        for (second_group, second_position) in positions.iter().enumerate().skip(first_group + 1) {
            let mut first = [0; 16];
            let mut second = [0; 16];
            first[*first_position] = 1;
            second[*second_position] = 1;
            let comparison = match to_1c_bytes(first).cmp(&to_1c_bytes(second)) {
                Ordering::Less => -1,
                Ordering::Equal => 0,
                Ordering::Greater => 1,
            };
            actual.push(format!("groups.{first_group}.{second_group}\t{comparison}"));
        }
    }
    assert_eq!(
        actual.join("\n"),
        include_str!(
            "../../../../docs/research/fixtures/document-point-in-time/group-order.platform.txt"
        )
        .trim()
    );
}
