import os

# Delete the files first.
for dirpath, dirnames, filenames in os.walk(os.getcwd()):
    for each_file in filenames:
        if each_file.endswith('.pyc'):
            if os.path.exists(os.path.join(dirpath, each_file)):
                os.remove(os.path.join(dirpath, each_file))
