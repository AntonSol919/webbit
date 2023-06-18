#!/bin/bash
if ! [ -x "$(command -v lk )" ]; then 
    echo `lk` is missing - https://github.com/AntonSol919/linkspace
    exit 1
fi

lk datapoint --data $1 | \
    lk collect --create [epoch] webbit::/${2:-1} --ctag data | \
    lk --init save -f | \
    lk p [hash:str] | \
    tail -1

