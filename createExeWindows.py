from distutils.core import setup
import py2exe
import os
import matplotlib as mpl

# delete all pyc files
for dirpath, dirnames, filenames in os.walk(os.getcwd()):
	for each_file in filenames:
			if each_file.endswith('.pyc'):
					if os.path.exists(os.path.join(dirpath, each_file)):
							os.remove(os.path.join(dirpath, each_file))

# create list of plugins
plugin_directories = ['','/common','/common/multipleComparisonCorrections','/common/multipleComparisonCorrections/additional',
						'/groups', '/groups/effectSizeFilters','/groups/plots','/groups/plots/configGUI','/groups/statisticalTests',
						'/multiGroups', '/multiGroups/effectSizeFilters','/multiGroups/plots','/multiGroups/plots/configGUI', '/multiGroups/postHoc','/multiGroups/statisticalTests',
						'/samples','/samples/confidenceIntervalMethods','/samples/effectSizeFilters','/samples/plots','/samples/plots/configGUI','/samples/statisticalTests',
						'/samples/statisticalTests/additional','/samples/plots/examples']
plugin_files = []
for directory in plugin_directories:
	for files in os.listdir("./stamp/plugins" + directory):
			f1 = "./stamp/plugins" + directory + "/" + files
			if os.path.isfile(f1): # skip directories
					f2 = "library/stamp/plugins" + directory, [f1]
					plugin_files.append(f2)

# grab all additional resource or data files
icon_files = []
for f in os.listdir("./stamp/icons"):
		f1 = "./stamp/icons/" + f
		if os.path.isfile(f1): # skip directories
			f2 = "icons", [f1]
			icon_files.append(f2)

example_files = []
for f in os.listdir("examples"):
		f1 = "examples/" + f
		if os.path.isfile(f1): # skip directories
			f2 = "examples", [f1]
			example_files.append(f2)
		else:
			if f != '.svn':
				for g in os.listdir(f1):
					if os.path.isfile(f1 + '/' + g): # skip directories
						f2 = 'examples/' + f, [f1 + '/' + g]
						example_files.append(f2)
				
for f in os.listdir('examples'):
		f1 = 'examples/' + f
		if os.path.isfile(f1): # skip directories
			f2 = 'examples', [f1]
			example_files.append(f2)

data_files = []
for f in os.listdir("./stamp/data"):
		f1 = "./stamp/data/" + f
		if os.path.isfile(f1): # skip directories
			f2 = "library/stamp/data", [f1]
			data_files.append(f2)
				
root_files = ['LICENSE.txt', './windows/STAMP.exe.log', './windows/readme.txt', 'msvcp90.dll', './manual/STAMP_Users_Guide.pdf']

				
mpl_data_files = mpl.get_py2exe_datafiles()

# setup configuration
setup(
	name = "STAMP",
	version = "2.0.3",
	description = "Statistical analysis of taxonomic and functional profiles",
	author = "Donovan Parks",
	windows=[{"script":"STAMP.py", "icon_resources": [(1, "./stamp/icons/programIcon.ico")]}],
	options = 
			{
				"py2exe":
				{
					"unbuffered": True,
					"optimize": 2,
					"skip_archive": True,
					"includes": ["sip", "PyQt4", "sqlite3"],
					"packages": ["matplotlib","pytz","scipy","mpl_toolkits", "pyparsing", "biom", "pyqi"],
					"dll_excludes": ["libgdk_pixbuf-2.0-0.dll","libgdk-win32-2.0-0.dll", "libgobject-2.0-0.dll", "tcl84.dll", "tk84.dll"],
				}
			},
	zipfile = "library/",
	data_files = icon_files + example_files + data_files + root_files + plugin_files + mpl_data_files,
)