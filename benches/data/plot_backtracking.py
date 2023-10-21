import matplotlib.pyplot as plt
import pandas as pd
import numpy as np


def plot_decision_nodes_distribution():
    scenarios = [
        "Summer 32x32",
        "Castle 32x32",
        "Floorplan 32x32",
        "Castle 64x64",
        "Floorplan 64x64",
    ]
    decision_nodes = {
        "Summer 32x32": {2: 484, 3: 14, 4: 24, 5: 6, 40: 1},
        "Castle 32x32": {2: 454, 3: 175, 4: 1, 5: 1, 29: 1},
        "Floorplan 32x32": {2: 221, 3: 212, 4: 185, 5: 54, 6: 2, 56: 1},
        "Castle 64x64": {2: 1759, 3: 653, 4: 3, 5: 1, 29: 1},
        "Floorplan 64x64": {2: 928, 3: 843, 4: 780, 5: 113, 56: 1},
    }
    df = pd.DataFrame(decision_nodes).fillna(0)  # Fill missing values with 0
    df = df.sort_index(ascending=True)
    df = df.astype(int)  # Convert float values (due to NaN fill) to int
    print(df)

    # Extracting unique decision nodes
    unique_nodes = sorted(
        {node for scenario in decision_nodes for node in decision_nodes[scenario]}
    )

    # Data for the table
    table_data = []
    # Add header
    table_data.append(["Scenario"] + [str(node) for node in unique_nodes])
    # Add data for each scenario
    for scenario in scenarios:
        row = [scenario]
        for node in unique_nodes:
            row.append(decision_nodes[scenario].get(node, "-"))
        table_data.append(row)

    # Plotting
    fig, ax = plt.subplots(figsize=(10, 6))

    # Hide axes
    ax.axis("off")
    ax.axis("tight")

    # Create table and set properties
    table = ax.table(cellText=table_data, cellLoc="center", loc="center")
    table.auto_set_font_size(False)
    table.set_fontsize(10)
    table.auto_set_column_width(col=list(range(len(unique_nodes) + 1)))

    plt.title("Decision Nodes Distribution")
    plt.savefig("figs/decision-distrubtuion.eps")
    # for scenario in scenarios:
    #     x = list(decision_nodes[scenario].keys())
    #     y = list(decision_nodes[scenario].values())

    #     plt.bar(x, y)
    #     plt.title(f'Decision Nodes Distribution for {scenario}')
    #     plt.xlabel('Decision Node')
    #     plt.ylabel('Count')
    #     plt.show()


def plot_time_to_run():
    scenarios = [
        "Castle 32x32",
        "Floorplan 32x32",
        "Summer 32x32",
        "Castle 128x128",
        "Floorplan 128x128",
    ]
    strategies = [
        "Standard Backtracking",
        "Degree Based",
        "Fixed Node",
        "Proportional Exploration",
        "Random Restarts",
    ]
    times = {
        "Castle 32x32": [
            0.01135141,
            0.009897475,
            0.009322364999999999,
            0.010380870000000002,
            0.024476104999999998,
        ],
        "Floorplan 32x32": [
            0.016704165000000003,
            0.015594384999999999,
            0.015705025000000004,
            0.016720799999999997,
            0.022776255000000002,
        ],
        "Summer 32x32": [
            float("inf"),
            0.019203205000000004,
            0.2469833,
            0.0194842,
            0.028609144999999996,
        ],  # using infinity for "too long"
        "Castle 128x128": [
            float("inf"),
            1.0300022400000002,
            0.91773504,
            0.9867857899999999,
            float("inf"),
        ],
        "Floorplan 128x128": [
            float("inf"),
            1.05503605,
            1.0430785799999998,
            1.0903707299999998,
            float("inf"),
        ],
    }

    #     for scenario in scenarios:
    #         plt.bar(strategies, times[scenario], label=scenario)
    #         plt.title(f'Time to Run for {scenario}')
    #         plt.xlabel('Strategy')
    #         plt.ylabel('Time (s)')
    #         plt.xticks(rotation=45)
    #         plt.tight_layout()
    #         plt.show()
    # Sizes to consider
    sizes = ["32x32", "128x128"]

    # Define width for individual bars
    bar_width = 0.1

    for size in sizes:
        fig, ax = plt.subplots(figsize=(10, 6))
        # Scenarios filtered based on size
        relevant_scenarios = [s for s in scenarios if size in s]

        # Position of the main bars (strategies)
        r = np.arange(len(strategies))

        for idx, scenario in enumerate(relevant_scenarios):
            time_data = times[scenario]
            bars = ax.bar(
                r + idx * bar_width,
                time_data,
                color=plt.cm.Paired.colors[idx],
                width=bar_width,
                label=scenario,
            )

            # Annotate 'Too Long' for 'inf' values
            for bar, value in zip(bars, time_data):
                if value == float("inf"):
                    offset = (idx - 1) * 0.02
                    ax.text(
                        bar.get_x() + bar.get_width() / 2 + offset,
                        0.1,
                        "Too Long",
                        ha="center",
                        va="bottom",
                        rotation=90,
                        color="red",
                    )
                    # bar.set_color(
                    #     "gray"
                    # )  # Dim the color for 'inf' values for better distinction

        ax.set_title(size)
        ax.set_xticks(r + bar_width * ((len(relevant_scenarios) - 1) / 2))
        ax.set_xticklabels([scenario for scenario in relevant_scenarios if scenario in times], rotation=45, ha="right")
        ax.legend()
        if size == "32x32":
            ax.set_ylabel("Time (s)")

        plt.tight_layout()
        plt.savefig(f"figs/time-{size}.eps")


def plot_backtracks():
    scenarios = [
        "Castle 32x32",
        "Floorplan 32x32",
        "Summer 32x32",
        "Castle 128x128",
        "Floorplan 128x128",
    ]
    strategies = [
        "Standard Backtracking",
        "Degree Based",
        "Fixed Node",
        "Proportional Exploration",
        "Random Restarts",
    ]
    backtracks = {
        "Castle 32x32": [1.75, 1.35, 0.65, 1.35, 2.2],
        "Floorplan 32x32": [0.8, 0.35, 0.4, 0.55, 0.75],
        "Summer 32x32": [
            float("inf"),
            4,
            261.7,
            3.45,
            2.2,
        ],  # using infinity for "too long"
        "Castle 128x128": [float("inf"), 30, 29.6, 27.8, float("inf")],
        "Floorplan 128x128": [float("inf"), 19, 16.85, 18.15, float("inf")],
    }

    # for scenario in scenarios:
    #     plt.bar(strategies, backtracks[scenario], label=scenario)
    #     plt.title(f'Number of Backtracks/Restarts for {scenario}')
    #     plt.xlabel('Strategy')
    #     plt.ylabel('Backtracks/Restarts')
    #     plt.xticks(rotation=45)
    #     plt.tight_layout()
    #     plt.show()

    n_strategies = len(strategies)
    fig, axes = plt.subplots(1, n_strategies, figsize=(15, 5))

    for idx, strategy in enumerate(strategies):
        ax = axes[idx]
        strategy_backtracks = [backtracks[scenario][idx] for scenario in scenarios]
        ax.bar(
            scenarios, strategy_backtracks, color=plt.cm.Paired.colors[: len(scenarios)]
        )
        ax.set_title(strategy)
        ax.set_xticklabels(scenarios, rotation=45, ha="right")
        if idx == 0:
            ax.set_ylabel("Backtracks/Restarts")

    plt.tight_layout()
    # plt.show()
    plt.savefig("figs/backtracks.eps")


# Running all the plotting functions:
# plot_decision_nodes_distribution()
plot_time_to_run()
# plot_backtracks()
