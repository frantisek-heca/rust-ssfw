// iteruju nad split_whitespace slovama a nalepuju je na sebe dokud celkova delka nepresahne chunk_length
// pak vyresetuju a slepuju novy chunk po slovech
fn split_string2(s: &str, chunk_length: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut temp_string = Vec::new();
    let mut temp_length = 0;
    for w in s.split_whitespace() {
        temp_length += w.chars().count() + 1;
        temp_string.push(w);
        if temp_length > chunk_length {
            chunks.push(temp_string.join(" "));
            temp_length = 0;
            temp_string = Vec::new();
        }
    }
    chunks
}

// iteruju po znacich a zaroven mam stale pozici iteratoru a kdyz je pozice vetsi nez chunk_length, tak vytvorim do teto pozice
// novy chunk.. a jedu dal
// proste si zafixovavam posledni pozici iteratoru
fn split_string(s: &str, chunk_length: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut last_cut = 0;
    let mut max_chunk_position = chunk_length;
    for (i, c) in s.char_indices() {
        if i - last_cut > chunk_length && c.is_whitespace() {
            chunks.push(s[last_cut..i].to_string());
            last_cut = i + 1;
        }
    }
    chunks.push(s[last_cut..].to_string());
    chunks
}

use std::collections::HashMap;

fn translation_data_provider() -> HashMap<&'static str, Vec<&'static str>> {
    let mut data = HashMap::new();

    let names = vec!["Produkt 1", "Produkt 2", "Produkt 3"];
    data.insert("names", names);

    let description = vec!["Description 1", "Description 2", "Description 3"];
    data.insert("descriptions", description);

    let long_descriptions = vec![
        "Long descriptions 1",
        "Long descriptions 2",
        "Long descriptions 3",
    ];
    data.insert("long_descriptions", long_descriptions);
    data
}

fn utils() {
    dbg!(split_string2("babička šla pro chleba a potkala lišku", 10));
    dbg!(split_string("babička šla pro chleba a potkala lišku", 10));
}