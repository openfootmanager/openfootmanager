#!/usr/bin/env bash
#
# Drives `mutation-slot.sh`, which decides what each weekly mutation-testing job mutates.
#
# The scheduled job only runs once a week and takes hours, so a slip in this arithmetic costs
# a week to notice: two jobs on one slot, a mutant no slot ever reaches, or a slot too big to
# finish in time. The first two are what the old `--shard` rotation did. Its slot count moved
# with the mutant counts (`ofm_core` went from 29 to 32 shards between 11 and 18 Oct 2026 and the
# rotation jumped back seven slots), and a shard is a position in cargo-mutants' list, so one
# added mutant reshuffled every shard after it. These cases pin the bucket planner that replaced
# it so neither can come back unnoticed.

set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
script="$here/mutation-slot.sh"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

passed=0
failed=0

record() {
    local ok="$1" description="$2" detail="$3"
    if [ "$ok" -eq 1 ]; then
        passed=$((passed + 1))
        printf '  ok    %s\n' "$description"
    else
        failed=$((failed + 1))
        printf '  FAIL  %s\n' "$description"
        printf '        %s\n' "$detail"
    fi
}

# expect_output <expected stdout> <what this proves> -- <arguments...>
expect_output() {
    local want="$1" description="$2"
    shift 3
    local output status
    # stdout alone: `select` reports its bucket's size on stderr, and only stdout reaches `--re`.
    output="$("$script" "$@" 2> "$scratch/stderr")"
    status=$?
    if [ "$status" -eq 0 ] && [ "$output" = "$want" ]; then
        record 1 "$description" ""
    else
        record 0 "$description" \
            "expected exit 0 and '$want', got exit $status and '$output' ($(cat "$scratch/stderr"))"
    fi
}

# expect_rejected <what this proves> <reason> -- <arguments...>
#
# The refusal must give <reason>: under `set -u` an unknown crate also fails, on an unbound array
# key, so a bare non-zero exit would pass with the guard that should catch it deleted.
expect_rejected() {
    local description="$1" reason="$2"
    shift 3
    local output status
    output="$("$script" "$@" 2>&1)"
    status=$?
    if [ "$status" -ne 0 ] && [[ "$output" == *"$reason"* ]]; then
        record 1 "$description" ""
    else
        record 0 "$description" \
            "expected a non-zero exit saying '$reason', got $status and '$output'"
    fi
}

# expect_selection_rejected <what this proves> <listing file> <reason> -- <arguments...>
#
# The refusal must give <reason>. Several guards overlap (an empty listing also selects nothing),
# so a refusal for another reason would hide the guard this case is about.
expect_selection_rejected() {
    local description="$1" listing="$2" reason="$3"
    shift 4
    local output status
    output="$("$script" "$@" < "$listing" 2>&1)"
    status=$?
    if [ "$status" -ne 0 ] && [[ "$output" == *"$reason"* ]]; then
        record 1 "$description" ""
    else
        record 0 "$description" \
            "expected a non-zero exit saying '$reason', got $status and '$output'"
    fi
}

# select_bucket <crate> <bucket> <listing file>: the patterns one bucket selects, one per line.
select_bucket() {
    "$script" select "$1" "$2" < "$3" 2>/dev/null
}

# unescape: turns selected patterns back into the mutant names they match exactly.
unescape() {
    sed -e 's/^\^//' -e 's/\$$//' -e 's/\\\(.\)/\1/g'
}

# bucket_of_each <crate> <buckets> <listing file>: "<bucket>\t<name without line:col>" for every
# mutant, so two listings of the same code can be compared once their line numbers move.
bucket_of_each() {
    local crate="$1" buckets="$2" listing="$3" bucket
    for ((bucket = 0; bucket < buckets; bucket++)); do
        select_bucket "$crate" "$bucket/$buckets" "$listing" | unescape |
            sed -E "s/^([^:]+):[0-9]+:[0-9]+: /$bucket\t\1: /"
    done | sort
}

# A listing shaped like `cargo mutants --list`: <path>:<line>:<column>: <mutation>. Thirty-nine
# mutants over five functions, the last three of them one mutation repeated at different lines
# of one function, as `replace += with -=` often is.
listing="$scratch/listing.txt"
for function in tackle pass shoot press; do
    for operator in '+ with -' '- with +' '* with /' '== with !=' '< with <=' '> with >='; do
        line=$((${#function} * 7 + ${#operator}))
        echo "crates/engine/src/play.rs:$line:9: replace $operator in $function"
    done
    for n in 1 2 3; do
        echo "crates/engine/src/play.rs:$((n * 100)):17: replace $function -> u8 with $n"
    done
done > "$listing"
for n in 1 2 3; do
    echo "crates/engine/src/play.rs:$((n * 11 + 400)):5: replace += with -= in recover"
done >> "$listing"
mutant_total="$(wc -l < "$listing")"

echo "mutation-slot.test: the rotation"

rotation="$("$script" rotation)"
slot_total="$(printf '%s\n' "$rotation" | wc -l)"
if [ "$slot_total" -eq 51 ]; then
    record 1 "the rotation is 51 slots: 1 + 2 + 4 + 44 buckets" ""
else
    record 0 "the rotation is 51 slots: 1 + 2 + 4 + 44 buckets" "got $slot_total slots:
$rotation"
fi

first_lines="$(printf '%s\n' "$rotation" | head -n 8)"
want_first='domain 0/1
db 0/2
db 1/2
engine 0/4
engine 1/4
engine 2/4
engine 3/4
ofm_core 0/44'
if [ "$first_lines" = "$want_first" ]; then
    record 1 "crates come in a fixed order, each bucket of a crate in turn" ""
else
    record 0 "crates come in a fixed order, each bucket of a crate in turn" "got:
$first_lines"
fi

# The old planner took live counts, and they were what moved the rotation under it.
expect_rejected "the rotation takes no mutant counts" "takes no arguments" -- \
    rotation domain=367 db=488 engine=1547 ofm_core=8360

echo "mutation-slot.test: picking a week's slots"

# Week 2964 is the one holding Sunday 25 Oct 2026, where the bucket rotation starts.
expect_output "domain 0/1" "the first week's first job is the first slot" -- pick 2964 0 8
expect_output "db 1/2" "the jobs of one week take consecutive slots" -- pick 2964 2 8
expect_output "ofm_core 1/44" "next week carries on where this week stopped" -- pick 2965 0 8
# 51 slots at 8 a week: week 6 of the cycle, job 3, is slot 51, the first slot again.
expect_output "domain 0/1" "the walk wraps to the start after exactly one cycle" -- pick 2970 3 8
expect_output "db 0/2" "and then carries straight on" -- pick 2970 4 8
expect_output "ofm_core 19/44" "a week far in the future still lands inside the rotation" -- \
    pick 4000 0 8

expect_rejected "a week before the rotation starts" "before the rotation starts" -- pick 2963 0 8
expect_rejected "more jobs a week than slots would test one slot twice" "only has 51 slots" -- \
    pick 2964 0 52
expect_rejected "a job number outside the week's jobs" "does not exist" -- pick 2964 8 8
# "2+1" and "0+1" rather than a word: bash arithmetic already rejects a word, so a word would
# pass whether or not the script checks. These are the ones it would quietly evaluate.
expect_rejected "a week that is not a plain number" "is not a week number" -- pick 2964+1 0 8
expect_rejected "a job number that is not a plain number" "is not a job number" -- \
    pick 2964 0+1 8
expect_rejected "a number of jobs that is not a plain number" "is not a number of jobs" -- \
    pick 2964 0 8+0
expect_rejected "a week with no jobs in it" "does not exist" -- pick 2964 0 0

echo "mutation-slot.test: a manual run of one crate"

expect_output "ofm_core 0/44" "a named crate starts at its own first bucket" -- first ofm_core
expect_output "domain 0/1" "a small crate is a single bucket" -- first domain
expect_rejected "a crate that is not in the rotation" "not in the rotation" -- \
    first ofm-cli

echo "mutation-slot.test: what one bucket selects"

all_selected="$(for bucket in 0 1 2 3; do select_bucket engine "$bucket/4" "$listing"; done)"
selected_total="$(printf '%s\n' "$all_selected" | grep -c .)"
distinct_total="$(printf '%s\n' "$all_selected" | sort -u | grep -c .)"
if [ "$selected_total" -eq "$mutant_total" ] && [ "$distinct_total" -eq "$mutant_total" ] &&
    [ "$(printf '%s\n' "$all_selected" | unescape | sort)" = "$(sort "$listing")" ]; then
    record 1 "every mutant falls in exactly one bucket, repeats of one mutation included" ""
else
    record 0 "every mutant falls in exactly one bucket, repeats of one mutation included" \
        "$selected_total selected, $distinct_total distinct, $mutant_total listed"
fi

nonempty=0
for bucket in 0 1 2 3; do
    [ -n "$(select_bucket engine "$bucket/4" "$listing")" ] && nonempty=$((nonempty + 1))
done
if [ "$nonempty" -ge 3 ]; then
    record 1 "the mutants spread across the buckets rather than piling into one" ""
else
    record 0 "the mutants spread across the buckets rather than piling into one" \
        "only $nonempty of 4 buckets selected anything"
fi

# The three repeats of `replace += with -= in recover` differ only by their line. Keyed on the
# name alone they would always share a bucket, and in `ofm_core` such repeats made the largest
# bucket a quarter bigger.
repeat_buckets="$(bucket_of_each engine 4 "$listing" | grep 'in recover$' | cut -f1 | sort -u |
    wc -l)"
if [ "$repeat_buckets" -gt 1 ]; then
    record 1 "repeats of one mutation in one function do not all share a bucket" ""
else
    record 0 "repeats of one mutation in one function do not all share a bucket" \
        "all three landed in one bucket"
fi

# The same code a week later: every line moved down by 12, one mutant added at the top of the
# file and one removed from the middle. Under `--shard` that reshuffled every shard.
moved="$scratch/moved.txt"
{
    echo "crates/engine/src/play.rs:2:1: replace kick_off -> bool with true"
    grep -v ': replace \* with / in pass$' "$listing" |
        awk -F: 'BEGIN { OFS = ":" } { $2 = $2 + 12; print }'
} > "$moved"
before="$(bucket_of_each engine 4 "$listing" | grep -v 'in pass$\|kick_off')"
after="$(bucket_of_each engine 4 "$moved" | grep -v 'in pass$\|kick_off')"
if [ -n "$before" ] && [ "$before" = "$after" ]; then
    record 1 "a mutant keeps its bucket when lines move and other mutants come and go" ""
else
    record 0 "a mutant keeps its bucket when lines move and other mutants come and go" \
        "$(diff <(printf '%s\n' "$before") <(printf '%s\n' "$after") | head -n 6)"
fi

# Names carry regex metacharacters, and cargo-mutants matches `--re` against the whole name.
# `<` and `>` stay bare: `\<` and `\>` are word boundaries in the regex crate, not literals.
odd="$scratch/odd.txt"
echo 'crates/domain/src/a.rs:3:9: replace <impl Fn(&[u8])>::call -> Option<usize> with Some(0) in {x}|y*z+q? ^$#~' > "$odd"
expect_output '^crates/domain/src/a\.rs:3:9: replace <impl Fn\(\&\[u8\]\)>::call \-> Option<usize> with Some\(0\) in \{x\}\|y\*z\+q\? \^\$\#\~$' \
    "a name's regex metacharacters are escaped, so it selects exactly that mutant" -- \
    select domain 0/1 < "$odd"

# Real `ofm_core` names from develop on 21 Oct 2026, pinned to the buckets they hash to. The
# cases above check properties, which an awk that hashed differently would satisfy just as
# well, and moving every mutant to another bucket would silently break the cycle that is in
# progress. The runners' `awk` is mawk, and gawk and mawk agree on these only because the script
# hashes bytes under `LC_ALL=C`: in a UTF-8 locale gawk counts the accented letters as one
# character each. Hence names with non-ASCII bytes and regex metacharacters, and one mutation
# repeated three times in one function.
golden="$scratch/golden.txt"
cat > "$golden" <<'NAMES'
crates/ofm_core/src/advance_results.rs:41:5: replace collect_advance_results -> Vec<AdvanceMatchResult> with vec![Default::default()]
crates/ofm_core/src/world/mod.rs:243:9: delete match arm "São Paulo" | "Rio" | "Belo Horizonte" | "Santos" | "Campinas" | "Bragantino" | "Juiz de Fora" | "Vitória" in brazil_state_region
crates/ofm_core/src/generator/clubs.rs:348:13: delete match arm 'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' | 'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' in ascii_letters
crates/ofm_core/src/advance_results.rs:53:12: delete ! in collect_advance_results
crates/ofm_core/src/advance_results.rs:53:36: delete ! in collect_advance_results
crates/ofm_core/src/advance_results.rs:84:32: delete ! in collect_advance_results
NAMES
want_golden="$(printf '%s\n' \
    '28	crates/ofm_core/src/advance_results.rs: replace collect_advance_results -> Vec<AdvanceMatchResult> with vec![Default::default()]' \
    '4	crates/ofm_core/src/world/mod.rs: delete match arm "São Paulo" | "Rio" | "Belo Horizonte" | "Santos" | "Campinas" | "Bragantino" | "Juiz de Fora" | "Vitória" in brazil_state_region' \
    '5	crates/ofm_core/src/generator/clubs.rs: delete match arm '"'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' | 'à' | 'á' | 'â' | 'ã' | 'ä' | 'å'"' in ascii_letters' \
    '6	crates/ofm_core/src/advance_results.rs: delete ! in collect_advance_results' \
    '7	crates/ofm_core/src/advance_results.rs: delete ! in collect_advance_results' \
    '8	crates/ofm_core/src/advance_results.rs: delete ! in collect_advance_results')"
got_golden="$(bucket_of_each ofm_core 44 "$golden")"
if [ "$got_golden" = "$want_golden" ]; then
    record 1 "real mutant names land in the buckets they have always landed in" ""
else
    record 0 "real mutant names land in the buckets they have always landed in" \
        "$(diff <(printf '%s\n' "$want_golden") <(printf '%s\n' "$got_golden"))"
fi

# Real `cargo mutants --list` output for the same twelve `domain` mutants, once as CI produces
# it and once plain. `dtolnay/rust-toolchain` exports CARGO_TERM_COLOR=always for every later
# step, and cargo-mutants 25.3.1 then colours the function and the replacement even into a file.
# Patterns built from those bytes matched nothing: the 25 Oct 2026 run failed in every slot with
# "patterns match 0". A coloured listing must select what a plain one does.
fixtures="$here/tests/mutation-slot"
coloured_patterns="$(select_bucket domain 0/1 "$fixtures/list-coloured.txt")"
plain_patterns="$(select_bucket domain 0/1 "$fixtures/list-plain.txt")"
if [ -n "$plain_patterns" ] && [ "$coloured_patterns" = "$plain_patterns" ] &&
    [ "$(printf '%s\n' "$coloured_patterns" | unescape)" = "$(cat "$fixtures/list-plain.txt")" ]; then
    record 1 "a coloured listing selects the plain names cargo-mutants matches against" ""
else
    record 0 "a coloured listing selects the plain names cargo-mutants matches against" \
        "$(printf '%s\n' "$coloured_patterns" | head -n 2 | cat -v)"
fi
if [ "$(bucket_of_each ofm_core 44 "$fixtures/list-coloured.txt")" = \
    "$(bucket_of_each ofm_core 44 "$fixtures/list-plain.txt")" ]; then
    record 1 "colour does not move a mutant to another bucket" ""
else
    record 0 "colour does not move a mutant to another bucket" \
        "$(diff <(bucket_of_each ofm_core 44 "$fixtures/list-coloured.txt" | cat -v) \
            <(bucket_of_each ofm_core 44 "$fixtures/list-plain.txt"))"
fi

echo "mutation-slot.test: selections that cannot be trusted"

empty="$scratch/empty.txt"
: > "$empty"
expect_selection_rejected "an empty listing, which is what a failed listing looks like" \
    "$empty" "is empty" -- select domain 0/1

garbled="$scratch/garbled.txt"
{
    head -n 3 "$listing"
    echo "warning: unused manifest key: package.mutants"
} > "$garbled"
expect_selection_rejected "a line that is not a mutant name" "$garbled" "not a mutant name" -- \
    select engine 0/4

# With no `--re` at all, cargo-mutants mutates the whole crate: a bucket that selects nothing
# must stop the job, not run every mutant.
lone="$scratch/lone.txt"
head -n 1 "$listing" > "$lone"
lone_empty=0
for bucket in 0 1 2 3; do
    refusal="$("$script" select engine "$bucket/4" < "$lone" 2>&1 > /dev/null)"
    if [[ "$refusal" == *"selects no mutants"* ]]; then
        lone_empty=$((lone_empty + 1))
    fi
done
if [ "$lone_empty" -eq 3 ]; then
    record 1 "a bucket that selects nothing is refused" ""
else
    record 0 "a bucket that selects nothing is refused" \
        "a one-mutant listing was refused by $lone_empty of 4 buckets, expected 3"
fi

# 20 seconds a mutant for `domain` and a 12,000-second budget: 600 mutants fill its one bucket
# exactly, and 601 must fail loudly rather than plan a slot that cannot finish.
full="$scratch/full.txt"
for n in $(seq 1 600); do
    echo "crates/domain/src/a.rs:$n:5: replace f$n -> u8 with 0"
done > "$full"
if "$script" select domain 0/1 < "$full" > /dev/null 2>&1; then
    record 1 "a bucket that exactly fills the budget runs" ""
else
    record 0 "a bucket that exactly fills the budget runs" "600 domain mutants were refused"
fi
echo "crates/domain/src/a.rs:601:5: replace f601 -> u8 with 0" >> "$full"
expect_selection_rejected "one mutant over the budget fails loudly instead of overrunning" \
    "$full" "raise BUCKETS[domain]" -- select domain 0/1

expect_selection_rejected "a bucket count that is not the crate's" "$listing" "has 4 buckets" -- \
    select engine 0/5
expect_selection_rejected "a bucket past the last one" "$listing" "does not exist" -- \
    select engine 4/4
expect_selection_rejected "a bucket that is not <index>/<count>" "$listing" \
    "is not <bucket>/<buckets>" -- select engine 0
expect_selection_rejected "a crate that is not in the rotation" "$listing" \
    "not in the rotation" -- \
    select ofm-cli 0/1
expect_rejected "an unknown command" "unknown command" -- shuffle

echo ""
if [ "$failed" -ne 0 ]; then
    echo "mutation-slot.test: $failed of $((passed + failed)) cases failed" >&2
    exit 1
fi

echo "mutation-slot.test: all $passed cases behaved as expected"
