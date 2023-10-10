import matplotlib.pyplot as plt

def plot_decision_nodes_distribution():
    scenarios = ["Summer 32x32", "Castle 32x32", "Floorplan 32x32", "Castle 64x64", "Floorplan 64x64"]
    decision_nodes = {
        "Summer 32x32": {2: 484, 3: 14, 4: 24, 5: 6, 40: 1},
        "Castle 32x32": {2: 454, 3: 175, 4: 1, 5: 1, 29: 1},
        "Floorplan 32x32": {2: 221, 3: 212, 4: 185, 5: 54, 6: 2, 56: 1},
        "Castle 64x64": {2: 1759, 3: 653, 4: 3, 5: 1, 29: 1},
        "Floorplan 64x64": {2: 928, 3: 843, 4: 780, 5: 113, 56: 1},
    }

    for scenario in scenarios:
        x = list(decision_nodes[scenario].keys())
        y = list(decision_nodes[scenario].values())

        plt.bar(x, y)
        plt.title(f'Decision Nodes Distribution for {scenario}')
        plt.xlabel('Decision Node')
        plt.ylabel('Count')
        plt.show()

def plot_time_to_run():
    scenarios = ["Castle 32x32", "Floorplan 32x32", "Summer 32x32", "Castle 128x128", "Floorplan 128x128"]
    strategies = ["Standard Backtracking", "Degree Based", "Fixed Node", "Proportional Exploration", "Random Restarts"]
    times = {
        "Castle 32x32": [float('inf'), 0.009897475, 0.009322364999999999, 0.010380870000000002, 0.024476104999999998],
        "Floorplan 32x32": [0.016704165000000003, 0.015594384999999999, 0.015705025000000004, 0.016720799999999997, 0.022776255000000002],
        "Summer 32x32": [float('inf'), 0.019203205000000004, 0.2469833, 0.0194842, 0.028609144999999996],  # using infinity for "too long"
        "Castle 128x128": [float('inf'), 1.0300022400000002, 0.91773504, 0.9867857899999999, float('inf')],
        "Floorplan 128x128": [0.93074476, 1.05503605, 1.0430785799999998, 1.0903707299999998, float('inf')],
    }

    for scenario in scenarios:
        plt.bar(strategies, times[scenario], label=scenario)
        plt.title(f'Time to Run for {scenario}')
        plt.xlabel('Strategy')
        plt.ylabel('Time (s)')
        plt.xticks(rotation=45)
        plt.tight_layout()
        plt.show()

def plot_backtracks():
    scenarios = ["Castle 32x32", "Floorplan 32x32", "Summer 32x32", "Castle 128x128", "Floorplan 128x128"]
    strategies = ["Standard Backtracking", "Degree Based", "Fixed Node", "Proportional Exploration", "Random Restarts"]
    backtracks = {
        "Castle 32x32": [float('inf'), 1.35, 0.65, 1.35, 2.2],
        "Floorplan 32x32": [0.8, 0.35, 0.4, 0.55, 0.75],
        "Summer 32x32": [float('inf'), 4, 261.7, 3.45, 2.2],  # using infinity for "too long"
        "Castle 128x128": [float('inf'), 30, 29.6, 27.8, float('inf')],
        "Floorplan 128x128": [15.6, 19, 16.85, 18.15, float('inf')],
    }

    for scenario in scenarios:
        plt.bar(strategies, backtracks[scenario], label=scenario)
        plt.title(f'Number of Backtracks/Restarts for {scenario}')
        plt.xlabel('Strategy')
        plt.ylabel('Backtracks/Restarts')
        plt.xticks(rotation=45)
        plt.tight_layout()
        plt.show()

# Running all the plotting functions:
plot_decision_nodes_distribution()
plot_time_to_run()
plot_backtracks()
