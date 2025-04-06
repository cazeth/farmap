use chrono::NaiveDate;
use criterion::{criterion_group, criterion_main, Criterion};
use farmap::{UserCollection, UsersSubset};

fn bench_spam_scores(c: &mut Criterion) {
    let home_dir = std::env::var("HOME").unwrap();
    let file_path = home_dir + "/.local/share/farmap/spam_2025-01-21.jsonl";
    let collection = UserCollection::create_from_file_and_collect_non_fatal_errors(&file_path)
        .unwrap()
        .0;

    c.bench_function("create collection", |b| {
        b.iter(|| UserCollection::create_from_file_and_collect_non_fatal_errors(&file_path))
    });

    c.bench_function("create set", |b| b.iter(|| UsersSubset::from(&collection)));
    let set = UsersSubset::from(&collection);
    run_benchmarks_on_set(c, &set, "full set");

    let filtered_set = set.filtered(|user| user.fid() < 10_000);

    run_benchmarks_on_set(c, &filtered_set, "fid < 10_000");
}

fn run_benchmarks_on_set(c: &mut Criterion, set: &UsersSubset, name: &str) {
    let mut group = c.benchmark_group(name);

    group.bench_function("current_spam_score_count", |b| {
        b.iter(|| set.current_spam_score_count())
    });

    group.bench_function("current_spam_score_distribution", |b| {
        b.iter(|| set.current_spam_score_distribution())
    });

    group.bench_function("spam score at date", |b| {
        b.iter(|| set.spam_score_count_at_date(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()))
    });

    group.bench_function("weekly spam score counts", |b| {
        b.iter(|| set.weekly_spam_score_counts())
    });

    group.bench_function("weekly spam score distributions", |b| {
        b.iter(|| set.weekly_spam_score_distributions())
    });

    group.bench_function("count updates", |b| b.iter(|| set.count_updates()));

    group.bench_function("fid lookup", |b| b.iter(|| set.user(11720).or(None)));

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default().noise_threshold(0.05);
    targets = bench_spam_scores
);
criterion_main!(benches);
