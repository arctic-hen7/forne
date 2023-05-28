// An implementation of the SuperMemo v2 algorithm.
//
// Algorithm SM-2, (C) Copyright SuperMemo World, 1991. (https://www.supermemo.com)

const RESPONSES = [ "0", "1", "2", "3", "4", "5" ];

fn get_weight(data, difficult) {
    // Cards that are not yet ready to review will be excluded (unless they're difficult)
    if difficult {
        return 2.0;
    } else if data.next_review <= get_seconds_since_epoch() {
        // Once a card is ready to review, it should be reviewed
        return 1.0;
    } else {
        return 0.0;
    }
}

fn adjust_card(res, data, difficult) {
    let quality = parse_int(res);

    // Any cards the user is failing should be repeated zealously until they get them consistently right
    if quality < 4 {
        difficult = true;
    } else {
        difficult = false;
    }

    // Taken from https://stackoverflow.com/questions/49047159/spaced-repetition-algorithm-from-supermemo-sm-2#49047160
    data.easiness = max(1.3, data.easiness + 0.1 - (5.0 - quality) * (0.08 + (5.0 - quality) * 0.02));

    if (quality < 3) {
        data.repetitions = 0;
    } else {
        data.repetitions += 1;
    }

    if (data.repetitions <= 1) {
        data.interval = 1;
    } else if (data.repetitions == 2) {
        data.interval = 6;
    } else {
        data.interval = round(data.interval * data.easiness);
    }

    let now = get_seconds_since_epoch();
    let seconds_in_day = 60 * 60 * 24;
    data.next_review = now + seconds_in_day * data.interval;

    return [data, difficult];
}
fn get_default_metadata() {
    return #{
        repetitions: 0,
        easiness: 2.5,
        interval: 1,
        // First review can be immediate
        next_review: get_seconds_since_epoch(),
    };
}

// Utility functions
fn max(x, y) {
    if x > y {  return x; } else { return y; }
}
