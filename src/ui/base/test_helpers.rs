#![cfg(test)]
pub fn lore_ipsum_lines(n: usize) -> Vec<String> {
    let sentence = lipsum::lipsum_words(n);
    let words: Vec<&str> = sentence.split_ascii_whitespace().collect();
    words.iter().map(|&x| x.into()).collect()
}
