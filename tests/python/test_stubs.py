"""Test that type stubs are working correctly."""

import subprocess
import sys
import tempfile
from pathlib import Path


def test_pyi_file_exists():
    """Verify that __init__.pyi exists and contains expected exports."""
    import trace_parser
    package_dir = Path(trace_parser.__file__).parent
    pyi_file = package_dir / "__init__.pyi"
    
    assert pyi_file.exists(), f"__init__.pyi not found in {package_dir}"
    
    content = pyi_file.read_text()
    
    # Проверка что экспорты есть
    expected_exports = [
        "Trace",
        "TraceSchedSwitch",
        "parse_trace",
    ]
    
    for export in expected_exports:
        assert export in content, f"{export} not found in __init__.pyi"


def test_all_exports_match():
    """Verify __all__ in __init__.py matches __init__.pyi."""
    import trace_parser
    package_dir = Path(trace_parser.__file__).parent
    
    py_content = (package_dir / "__init__.py").read_text()
    pyi_content = (package_dir / "__init__.pyi").read_text()
    
    # Извлекаем __all__ из обоих файлов
    import re
    py_all = re.search(r'__all__ = \((.*?)\)', py_content, re.DOTALL)
    pyi_all = re.search(r'__all__ = \((.*?)\)', pyi_content, re.DOTALL)
    
    assert py_all is not None, "__all__ not found in __init__.py"
    assert pyi_all is not None, "__all__ not found in __init__.pyi"
    
    # Сравниваем списки
    py_exports = [x.strip().strip('"\'') for x in py_all.group(1).split(',') if x.strip()]
    pyi_exports = [x.strip().strip('"\'') for x in pyi_all.group(1).split(',') if x.strip()]
    
    assert set(py_exports) == set(pyi_exports), \
        f"__all__ mismatch: py={py_exports}, pyi={pyi_exports}"


def test_all_exports_are_tuple():
    """Verify __all__ is a tuple (not list) in both .py and .pyi."""
    import trace_parser
    package_dir = Path(trace_parser.__file__).parent
    
    py_content = (package_dir / "__init__.py").read_text()
    pyi_content = (package_dir / "__init__.pyi").read_text()
    
    # Проверяем что __all__ объявлен как tuple (круглые скобки)
    assert "__all__ = (" in py_content, "__all__ should be a tuple in __init__.py"
    assert "__all__ = (" in pyi_content, "__all__ should be a tuple in __init__.pyi"


def test_mypy_type_checking():
    """Verify mypy can type-check imports from trace_parser."""
    # Создаём тестовый файл
    test_code = '''
from trace_parser import Trace, TraceSchedSwitch, parse_trace

# Проверка что классы имеют правильные типы
def process_trace(t: Trace) -> str:
    return t.thread_name

def process_sched(s: TraceSchedSwitch) -> str:
    return s.prev_comm  # TraceSchedSwitch имеет prev_comm, не timestamp

# Проверка что parse_trace возвращает правильный тип
event = parse_trace("some-line")
if event is not None:
    name: str = event.thread_name  # Trace имеет thread_name
'''
    
    with tempfile.NamedTemporaryFile(mode='w', suffix='.py', delete=False) as f:
        f.write(test_code)
        temp_file = Path(f.name)
    
    try:
        # Запускаем mypy в режиме strict
        result = subprocess.run(
            [sys.executable, '-m', 'mypy', '--strict', str(temp_file)],
            capture_output=True,
            text=True,
            cwd=Path(__file__).parent.parent
        )
        
        # mypy должен пройти без ошибок
        assert result.returncode == 0, f"mypy failed:\n{result.stdout}\n{result.stderr}"
    finally:
        temp_file.unlink()


def test_copy_and_deepcopy():
    """Verify that __copy__ and __deepcopy__ work correctly."""
    import copy
    from trace_parser import Trace
    
    # Создаём тестовый объект
    trace = Trace(
        thread_name="bash",
        tid=1234,
        tgid=1234,
        cpu=0,
        flags="....",
        timestamp=12345.678901,
        event_name="sched_switch",
        payload_raw="test"
    )
    
    # Проверяем copy
    trace_copy = copy.copy(trace)
    assert trace_copy.thread_name == trace.thread_name
    assert trace_copy is not trace  # Different object
    
    # Проверяем deepcopy
    trace_deep = copy.deepcopy(trace)
    assert trace_deep.thread_name == trace.thread_name
    assert trace_deep is not trace  # Different object


def test_repr_and_str():
    """Verify that __repr__ and __str__ work correctly."""
    from trace_parser import Trace
    
    trace = Trace(
        thread_name="bash",
        tid=1234,
        tgid=1234,
        cpu=0,
        flags="....",
        timestamp=12345.678901,
        event_name="sched_switch",
        payload_raw="test"
    )
    
    # Проверяем __repr__
    repr_str = repr(trace)
    assert "Trace" in repr_str
    assert "bash" in repr_str
    
    # Проверяем __str__
    str_str = str(trace)
    assert "bash" in str_str
    assert "sched_switch" in str_str


def test_equality():
    """Verify that __eq__ works correctly."""
    from trace_parser import Trace
    
    trace1 = Trace(
        thread_name="bash",
        tid=1234,
        tgid=1234,
        cpu=0,
        flags="....",
        timestamp=12345.678901,
        event_name="sched_switch",
        payload_raw="test"
    )
    
    trace2 = Trace(
        thread_name="bash",
        tid=1234,
        tgid=1234,
        cpu=0,
        flags="....",
        timestamp=12345.678901,
        event_name="sched_switch",
        payload_raw="test"
    )
    
    trace3 = Trace(
        thread_name="bash",
        tid=1234,
        tgid=1234,
        cpu=0,
        flags="....",
        timestamp=12345.678901,
        event_name="sched_switch",
        payload_raw="different"
    )
    
    assert trace1 == trace2  # Same values
    assert trace1 != trace3  # Different payload
