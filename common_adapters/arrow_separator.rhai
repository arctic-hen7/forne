let pairs = [];

let cap_groups = captures(`(?m)^(.*) -> (.*)$`, SOURCE);
for caps in cap_groups {
    pairs.push([caps[1], caps[2]]);
}

return pairs;
