from cx_Freeze import setup,Executable

includefiles = ['README.md', 'state.json']
includes = []
excludes = []
packages = []

setup(
    name = 'SoQLStudio',
    version = '0.1',
    description = '',
    author = '',
    author_email = '',
    options = {'build_exe': {'includes':includes,'excludes':excludes,'packages':packages,'include_files':includefiles}}, 
    executables = [Executable('app.py')]
)
