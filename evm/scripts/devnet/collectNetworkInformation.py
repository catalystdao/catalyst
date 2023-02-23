import os
import json


# Finding tmp folder
tmp_path = "/tmp/"
tmp_folder = []
files = os.listdir(tmp_path)
keyword = "polymer-devnet-"
for file in files:
    if not os.path.isfile(os.path.join(tmp_path, file)):
        if keyword in file:
            print(f"Found temp folder at {file}")
            tmp_folder.append(os.path.join(tmp_path, file))

assert len(tmp_folder) == 1, f"Found {len(tmp_folder)}, expected 1"
tmp_folder = tmp_folder[0]

if not os.path.exists(os.path.join(tmp_folder, "catalyst")):
    os.mkdir(os.path.join(tmp_folder, "catalyst"))

# Reading ETH config

ETHCONFIG = {}
assert os.path.isfile(
    os.path.join(tmp_folder, "eth", "chain-config.json")
), f'Could not find eth chain-config.json at: {os.path.join(tmp_folder, "eth", "chain-config.json")}'

with open(os.path.join(tmp_folder, "eth", "chain-config.json"), "r") as f:
    ETHCONFIG = json.load(f)


# Reading BSC config

BSCCONFIG = {}
assert os.path.isfile(
    os.path.join(tmp_folder, "bsc", "chain-config.json")
), f'Could not find bsc chain-config.json at: {os.path.join(tmp_folder, "bsc", "chain-config.json")}'

with open(os.path.join(tmp_folder, "bsc", "chain-config.json"), "r") as f:
    BSCCONFIG = json.load(f)


catalystConfig = os.path.join(tmp_folder, "catalyst", "contracts.json")
if not os.path.isfile(catalystConfig):
    with open(catalystConfig, "w") as f:
        json.dump({"eth": {}, "bsc": {}}, f)


def modifyConfig(keys, value):
    with open(catalystConfig, "r") as f:
        cc = json.load(f)

    assert len(keys) <= 2, "Keylength max 2"

    if len(keys) == 1:
        cc[keys[0]] = value
    if len(keys) == 2:
        cc[keys[0]][keys[1]] = value

    with open(catalystConfig, "w") as f:
        json.dump(cc, f)


def getConfig():
    with open(catalystConfig, "r") as f:
        return json.load(f)


def stringToHex(string, size=64):
    bytestring = bytes(string, "utf-8")
    hexstring = bytestring.hex()
    
    return "0x" + hexstring + "".join(["0" for _ in range(len(hexstring), size)])
