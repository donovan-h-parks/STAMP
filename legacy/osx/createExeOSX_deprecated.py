from setuptools import setup
import os
import matplotlib as mpl

print 'Setup on OS X is current deprecated.'
sys.exit()

os.system('rm -rf build dist')

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
	for files in os.listdir("plugins" + directory):
			f1 = "plugins" + directory + "/" + files
			if os.path.isfile(f1): # skip directories
					f2 = "./lib/python2.6/site-packages/plugins" + directory, [f1]
					plugin_files.append(f2)

# grab all additional resource or data files
icon_files = []
for files in os.listdir('icons'):
		f1 = 'icons/' + files
		if os.path.isfile(f1): # skip directories
				f2 = './icons', [f1]
				icon_files.append(f2)
				
data_files = []
for files in os.listdir('data'):
		f1 = 'data/' + files
		if os.path.isfile(f1): # skip directories
				f2 = './data', [f1]
				data_files.append(f2)

root_files = ['/opt/local/lib/Resources/qt_menu.nib']

# setup configuration
setup(
	app = ['STAMP.py'],
	name = 'STAMP',
	version = '2.0.1',
	description = 'Statistical analysis of metagenomic profiles',
	author = 'Donovan Parks',
	options = 
			{
				'py2app':
				{
					'argv_emulation': False,
					'iconfile': './icons/STAMP.icns',
					'includes': ['sip', 'scipy'],
					'packages': ['PyQt4','matplotlib', 'mpl_toolkits', 'pyparsing'],
					'optimize': 2,
					'excludes': ['plugins'],
					'dylib_excludes': ['libQtXmlPatterns.4.dylib','libQtXml.4.dylib','libQtWebKit.4.dylib','libQtTest.4.dylib','libQtSvg.4.dylib','libQtSql.4.dylib','libQtScriptTools.4.dylib','libQtScript.4.dylib','libQtOpenGL.4.dylib','libQtNetwork.4.dylib','libQtMultimedia.4.dylib','libQtHelp.4.dylib','libQtDesigner.4.dylib','libQtDeclarative.4.dylib','libQtCLucene.4.dylib'],
					'plist': {'CFBundleGetInfoString': 'STAMP',},
				}
			},
	setup_requires=['py2app'],
	data_files = plugin_files + root_files +  icon_files + data_files + plugin_files,
)

os.system('macdeployqt ./dist/STAMP.app')
os.system('cp license.txt ./dist/license.txt')
os.system('cp -r ./examples ./dist/')
os.system('cp ./UsersGuide/STAMP_Users_Guide_v2.0.0.pdf ./dist/')
print 'Creating ZIP file...'
os.system('zip -rqy9 ./dist/STAMP_2_rc3_SnowLeopard.zip .')
print 'Done.'
