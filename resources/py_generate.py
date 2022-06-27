import csv
import random
import json


def random_tx_type():
    val = random.randint(0, 4)
    if val == 0:
        return "deposit"
    if val == 1:
        return "withdrawal"
    if val == 2:
        return "dispute"
    if val == 3:
        return "resolve"
    if val == 4:
        return "chargeback"


def make_record() -> dict:
    tx_type = random_tx_type()
    if tx_type in ["deposit", "withdrawal"]:
        return {
            "type": tx_type,
            "client": random.randint(0, 10000),
            "tx": random.randint(0, 100000),
            "amount": random.random() * 100000,
        }
    else:
        return {
            "type": tx_type,
            "client": random.randint(0, 10000),
            "tx": random.randint(0, 100000),
            "amount": 0,
        }


if __name__ == "__main__":
    with open("demofile3.txt", "w") as f:
        r = make_record()
        writer = csv.DictWriter(f, fieldnames=list(r.keys()))
        writer.writeheader()
        for i in range(1_000_000):
            writer.writerow(make_record())
