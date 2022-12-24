from dataclasses import dataclass
import numpy as np
import matplotlib.pyplot as plt
from os.path import isdir, join
from os import mkdir
from tqdm import trange
from typing import Callable, List, Tuple
from random import randrange

from integer import Uint256
from functools import cache

def get_rel_error(val: int | float | None, target: int | float | None) -> float | None:

    if val is None or target is None:
        return None
    
    if val == 0 and target == 0:
        return 0

    return abs(2*(val - target)/(abs(val) + abs(target)))


@cache
def get_powers_of_2_x64(start: int = 0, stop: int = 256, step: int = 1) -> List[int]:

    return [int(2**x*2**64) for x in range(start, stop, step)]


@cache
def get_powers_of_2_minus_1_x64(start: int = 0, stop: int = 257, step: int = 1) -> List[int]:

    return [y-1 for y in get_powers_of_2_x64(start, stop, step)]


def remove_duplicates_and_sort(l: List[int]) -> List[int]:

    l = list(set(l))
    l.sort()

    return l


def get_all_combinations(l1: List[int], l2: List[int]) -> Tuple[List[int], List[int]]:

    l1_out = []
    l2_out = []

    for l1_el in l1:
        for l2_el in l2:
            l1_out.append(l1_el)
            l2_out.append(l2_el)
    

    return l1_out, l2_out

def filter_lists(l1: List[int], l2: List[int], condition: Callable[[int, int], bool]) -> Tuple[Tuple[List[int], List[int]], Tuple[List[int], List[int]]]:
    l1_true: List[int] = []
    l2_true: List[int] = []
    l1_false: List[int] = []
    l2_false: List[int] = []

    for l1_el, l2_el in zip(l1, l2):
        if condition(l1_el, l2_el):
            l1_true.append(l1_el)
            l2_true.append(l2_el)
        else:
            l1_false.append(l1_el)
            l2_false.append(l2_el)

    return (l1_true, l2_true), (l1_false, l2_false)

def sample_space(
    sample_count : int,
    range_start  : int,
    range_end    : int,
) -> List[int]:

    return [randrange(range_start, range_end) for _ in range(sample_count)]


def sample_2d_space(
        sample_count  : int,
        x_range_start : int,
        x_range_end   : int,
        y_range_start : int,
        y_range_end   : int,
        x_min_func    : Callable[[int], int] | None = None,
        x_max_func    : Callable[[int], int] | None = None,
        y_min_func    : Callable[[int], int] | None = None,
        y_max_func    : Callable[[int], int] | None = None,
    ) -> Tuple[List[int], List[int]]:

        x_values = []
        y_values = []
        
        # Sample x first, then y
        for _ in range(int(sample_count/2)):

            while True:
                x = randrange(x_range_start, x_range_end)
                try:
                    y = randrange(
                        max(y_min_func(x), y_range_start) if y_min_func is not None else y_range_start,
                        min(y_max_func(x), y_range_end) if y_max_func is not None else y_range_end
                    )
                except:
                    continue

                x_values.append(x)
                y_values.append(y)
                break

        for _ in range(int(sample_count/2)):

            while True:
                y = randrange(y_range_start, y_range_end)
                try:
                    x = randrange(
                        max(x_min_func(y), x_range_start) if x_min_func is not None else x_range_start,
                        min(x_max_func(y), x_range_end) if x_max_func is not None else x_range_end
                    )
                except:
                    continue

                x_values.append(x)
                y_values.append(y)
                break

        return x_values, y_values


def evaluate_samples(
    sampling_points: List[List[int]],
    impl_fn: Callable[..., Uint256 | None],
    target_fn: Callable[..., int | None],
) -> Tuple[np.ndarray, np.ndarray, np.ndarray]:

    points_count  = len(sampling_points[0])

    calc_values   = np.zeros(points_count, dtype='O')
    target_values = np.zeros(points_count, dtype='O')
    rel_error     = np.zeros(points_count, dtype='O')

    # NOTE: it's not possible to use numpy's transformations (e.g. summation of 2 numpy objects), as ints can exceed the maximum allowed
    for i in trange(points_count):

        calc_val = impl_fn(*[Uint256(points[i]) for points in sampling_points])
        calc_val = calc_val.value if calc_val is not None else None

        target_val = target_fn(*[points[i] for points in sampling_points])

        calc_values[i]   = calc_val
        target_values[i] = target_val
        rel_error[i]     = get_rel_error(calc_val, target_val)
    
    return calc_values, target_values, rel_error


@dataclass
class Evaluate1VarResult:
    calc_values   : np.ndarray
    target_values : np.ndarray
    rel_error     : np.ndarray
    min_error     : float | None
    max_error     : float | None
    avg_error     : float | None
    invalid_count : int


def evaluate_implementation_1_var(
    sampling_points: List[int],
    impl_fn: Callable[[Uint256], Uint256 | None],
    target_fn: Callable[[int], int | None],
    name: str | None = None,
    x_label: str = "x",
    y_label: str = "y",
    comp_plot_legend_loc: str | int = "best",
    error_plot_legend_loc: str | int = "best",
    show_plots: bool = True,
    save_plots: bool = False,
    save_plots_dir: str = "./plots"
) -> Evaluate1VarResult:

    samples_x = np.array(sampling_points, dtype='O')
    samples_x_scaled = samples_x / 2**64
    
    # Evaluate functions
    calc_values, target_values, rel_error = evaluate_samples([sampling_points], impl_fn, target_fn)

    # Process data
    target_valid_outputs   = target_values != np.array(None)
    target_invalid_outputs = target_valid_outputs == False
    calc_valid_outputs     = calc_values != np.array(None)
    calc_invalid_outputs   = calc_valid_outputs == False
    error_valid_outputs    = rel_error != np.array(None)
    error_invalid_outputs  = error_valid_outputs == False

    # Print errors
    min_error = None
    max_error = None
    avg_error = None

    if (error_valid_outputs.sum() > 0):
        min_error = rel_error[error_valid_outputs].min()
        max_error = rel_error[error_valid_outputs].max()
        avg_error = float(np.average(rel_error[error_valid_outputs]))
        print(f"Min rel error: {min_error}")
        print(f"Max rel error: {max_error}")
        print(f"Avg rel error: {avg_error}")
        print(f"Invalid points: {error_invalid_outputs.sum()} of {rel_error.shape[0]} => {error_invalid_outputs.sum()/rel_error.shape[0]*100}%")
    
    else:
        print("No valid points available for stats")


    # Handle plots
    if show_plots or save_plots:
        # Prepare output directory for plots
        if save_plots and not isdir(save_plots_dir):
            mkdir(save_plots_dir)

        save_name = name.replace("/", "") if name is not None else None

        # Plot functions comparison
        fig_func, ax = plt.subplots()
        ax.set_title(((name + " ") if name else "") + "Target vs. Implemented", fontsize=20)
        ax.grid(True, which='major')
        ax.set_axisbelow(True)
        ax.set_xlabel(x_label)
        ax.set_ylabel(y_label)


        # Target function - valid points
        ax.scatter(
            samples_x_scaled[target_valid_outputs],
            target_values[target_valid_outputs] / 2**64,
            c='black',
            s=5,
            label='Target'
        )

        # Target function - invalid points
        ax.scatter(
            samples_x_scaled[target_invalid_outputs],
            [0 for _ in range(target_invalid_outputs.sum())],
            edgecolors=None,
            c='orange',
            marker='+', # type: ignore
            s=5,
            label='Target NaN'
        )

        
        # Implemented function - valid points
        ax.scatter(
            samples_x_scaled[calc_valid_outputs],
            calc_values[calc_valid_outputs] / 2**64,
            c='dodgerblue',
            s=15,
            label="Impl."
        )

        # Implemented function - invalid points
        ax.scatter(
            samples_x_scaled[calc_invalid_outputs],
            [0 for _ in range(calc_invalid_outputs.sum())],
            c='red',
            marker='x', # type: ignore
            s=15,
            label="Impl. NaN"
        )
        
        ax.legend(loc=comp_plot_legend_loc)
        
        if save_plots:
            save_path = join(save_plots_dir, ((save_name + "_") if save_name else "") + "target_vs_impl.jpg")
            plt.savefig(save_path, dpi=300, bbox_inches='tight')


        # Plot relative error
        fig_error, ax = plt.subplots()
        ax.set_title(((name + " ") if name else "") + "Relative Error", fontsize=20)
        ax.grid(True, which='major')
        ax.set_axisbelow(True)
        ax.set_xlabel(x_label)
        ax.set_ylabel(y_label)


        # Valid points
        ax.scatter(
            samples_x_scaled[error_valid_outputs].astype('float'),
            rel_error[error_valid_outputs],
            c='darkviolet',
            s=15,
            label="Error"
        )

        # Invalid points
        ax.scatter(
            samples_x_scaled[error_invalid_outputs].astype('float'),
            [0 for _ in range(error_invalid_outputs.sum())],
            c='red',
            marker='x', # type: ignore
            s=15,
            label="Error NaN"
        )
        
        ax.legend(loc=error_plot_legend_loc)
        
        if save_plots:
            save_path = join(save_plots_dir, ((save_name + "_") if save_name else "") + "error.jpg")
            plt.savefig(save_path, dpi=300, bbox_inches='tight')
        
        
        if show_plots:
            plt.show()
        else:
            plt.close(fig=fig_func)
            plt.close(fig=fig_error)

    return Evaluate1VarResult(
        calc_values   = calc_values,
        target_values = target_values,
        rel_error     = rel_error,
        min_error     = min_error,
        max_error     = max_error,
        avg_error     = avg_error,
        invalid_count = error_invalid_outputs.sum()
    )

def evaluate_implementation_2_vars(
    sampling_points: Tuple[List[int], List[int]],
    impl_fn: Callable[[Uint256, Uint256], Uint256 | None],
    target_fn: Callable[[int, int], int | None],
    name: str | None = None,
    x_label: str = "x",
    y_label: str = "y",
    plot_azim: int = -60,
    plot_elev: int = 30,
    show_plots: bool = True,
    save_plots: bool = False,
    save_plots_dir: str = "./plots"
):
    # Prepare output directory for plots
    if save_plots and not isdir(save_plots_dir):
        mkdir(save_plots_dir)

    save_name = name.replace("/", "") if name is not None else None

    # Convert into numpy arrays to use it's indexing features
    # ! Specify the dtype as 'O' (object), as otherwise the values get casted to float, yielding inaccurate results
    samples_x = np.array(sampling_points[0], dtype='O')
    samples_y = np.array(sampling_points[1], dtype='O')

    # Scale samples for plotting 
    samples_x_scaled = samples_x / 2**64
    samples_y_scaled = samples_y / 2**64

    calc_values   = np.empty(samples_x.shape, dtype='O')
    target_values = np.empty(samples_x.shape, dtype='O')
    rel_error     = np.empty(samples_x.shape, dtype='O')

    # Evaluate samples    
    calc_values, target_values, rel_error = evaluate_samples(list(sampling_points), impl_fn, target_fn)

    # Plot target function
    fig_target, ax = plt.subplots(subplot_kw={"projection": "3d"}, figsize=(8, 7))
    ax.set_title(((name + " ") if name else "") + "Target Function", fontsize=20)
    ax.set_xlabel(x_label)
    ax.set_ylabel(y_label)
    ax.set_zlabel("Eval.")

    # Target function - valid points
    target_valid_outputs = target_values != np.array(None)
    ax.scatter(
        samples_x_scaled[target_valid_outputs],
        samples_y_scaled[target_valid_outputs],
        target_values[target_valid_outputs] / 2**64,
        c=target_values[target_valid_outputs] if target_valid_outputs.sum() == 0 else target_values[target_valid_outputs]/target_values[target_valid_outputs].max(),
        cmap=plt.get_cmap('viridis')
    )

    # Target function - invalid points
    target_invalid_outputs = target_valid_outputs == False
    ax.scatter(
        samples_x_scaled[target_invalid_outputs],
        samples_y_scaled[target_invalid_outputs],
        0,
        c='red',
        marker='x' # type: ignore
    )
    
    ax.view_init(azim=plot_azim, elev=plot_elev)
    if save_plots:
        save_path = join(save_plots_dir, ((save_name + "_") if save_name else "") + "target.jpg")
        plt.savefig(save_path, dpi=300, bbox_inches='tight')


    # Plot implemented function
    fig_impl, ax = plt.subplots(subplot_kw={"projection": "3d"}, figsize=(8, 7))
    ax.set_title(((name + " ") if name else "") + "Implemented Function", fontsize=20)
    ax.set_xlabel(x_label)
    ax.set_ylabel(y_label)
    ax.set_zlabel("Eval.")

    # Implemented function - valid points
    calc_valid_outputs = calc_values != np.array(None)
    ax.scatter(
        samples_x_scaled[calc_valid_outputs],
        samples_y_scaled[calc_valid_outputs],
        calc_values[calc_valid_outputs] / 2**64,
        c=calc_values[calc_valid_outputs] if calc_valid_outputs.sum() == 0 else calc_values[calc_valid_outputs]/calc_values[calc_valid_outputs].max(),
        cmap=plt.get_cmap('viridis')
    )

    # Implemented function - invalid points
    calc_invalid_outputs = calc_valid_outputs == False
    ax.scatter(
        samples_x_scaled[calc_invalid_outputs],
        samples_y_scaled[calc_invalid_outputs],
        0,
        c='red',
        marker='x' # type: ignore
    )

    ax.view_init(azim=plot_azim, elev=plot_elev)
    if save_plots:
        save_path = join(save_plots_dir, ((save_name + "_") if save_name else "") + "impl.jpg")
        plt.savefig(save_path, dpi=300, bbox_inches='tight')

    
    # Plot error
    fig_error, ax = plt.subplots(subplot_kw={"projection": "3d"}, figsize=(8, 7))
    ax.set_title(((name + " ") if name else "") + "Relative Error", fontsize=20)
    ax.set_xlabel(x_label)
    ax.set_ylabel(y_label)
    ax.set_zlabel("Error")

    # Valid points
    error_valid_outputs = rel_error != np.array(None)
    if error_valid_outputs.sum() > 0:
        max_error = rel_error[error_valid_outputs].max()
        min_error = rel_error[error_valid_outputs].min()
    else:
        max_error = None
        min_error = None

    ax.scatter(
        samples_x_scaled[error_valid_outputs],
        samples_y_scaled[error_valid_outputs],
        rel_error[error_valid_outputs],
        c=rel_error[error_valid_outputs] if max_error == 0 or max_error is None else rel_error[error_valid_outputs]/rel_error[error_valid_outputs].max(),
        cmap=plt.get_cmap('viridis')
    )

    # Invalid points
    error_invalid_outputs = error_valid_outputs == False
    ax.scatter(
        samples_x_scaled[error_invalid_outputs],
        samples_y_scaled[error_invalid_outputs],
        0,
        c='red',
        marker='x' # type: ignore
    )
    
    # Print errors
    if (error_valid_outputs.sum() > 0):
        avg_error = np.average(rel_error[error_valid_outputs])
        print(f"Min rel error: {min_error}")
        print(f"Max rel error: {max_error}")
        print(f"Avg rel error: {avg_error}")
        print(f"Invalid points: {error_invalid_outputs.sum()} of {rel_error.shape[0]} => {error_invalid_outputs.sum()/rel_error.shape[0]*100}%")
    
    else:
        print("No valid points available for stats")


    ax.view_init(azim=plot_azim, elev=plot_elev)
    if save_plots:
        save_path = join(save_plots_dir, ((save_name + "_") if save_name else "") + "error.jpg")
        plt.savefig(save_path, dpi=300, bbox_inches='tight')


    if show_plots:
        plt.show()
    else:
        plt.close(fig=fig_target)
        plt.close(fig=fig_impl)
        plt.close(fig=fig_error)
