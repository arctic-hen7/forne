let lines = SOURCE.split("\n");
let pairs = [];
for line in lines {
    if line == "" {
       continue;
    }
    let elems = line.split(" -> ");
    let question = elems[0];
    let answer = elems[1];

    pairs.push([question, answer]);
}

return pairs;
