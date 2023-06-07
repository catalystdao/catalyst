import argparse
import json

from poa_relayer import PoARelayer


def main():
    parser = argparse.ArgumentParser("proxy relayer")
    parser.add_argument(
        "config_location", nargs="?", help="The path to the config location", type=str
    )
    args = parser.parse_args()
    config_location = "./scripts/deploy_config.json"
    if args.config_location:
        config_location = args.config_location

    relayer = PoARelayer(config_name=config_location)
    relayer.backcheck()


if __name__ == "__main__":
    main()
