import os,json
from tkinter import Tk
from tkinter import filedialog
from requests import get
from time import sleep 
from shutil import rmtree

#确定选择文件夹
print("本程序没有任何校验数据功能，请确认已经将番剧存放在单独的目录，并且清楚会遭遇的任何风险！！！")
yn = input("是否开始选择番剧存放目录？(y/n)：")
if yn == "n":
    print("放弃选择，3秒后退出……")
    sleep(3)
    exit()
elif yn != "y":
    exit()


#打开选择文件夹对话框
Tk().withdraw()

#获得选择好的文件夹路径
folder = filedialog.askdirectory()

#若取消选择则退出
if len(folder) == 0:
    print("放弃选择，3秒后退出……")
    sleep(3)
    exit()

#更改工作目录为选择文件夹的父路径
main_dir = os.path.abspath(os.path.dirname(folder))
print(main_dir)
os.chdir(main_dir)

#借助bangumi.tv的API搜索番剧信息
fjnamess = input("从bangumi.tv搜索番剧名：")
url = "https://api.bgm.tv/search/subject/"+fjnamess+"?type=2&responseGroup=small"
headers = {'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_11_5) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/50.0.2661.102 Safari/537.36'}
fjname = get(url,headers=headers)

#格式化json
fjname_js = fjname.json()

data = json.dumps(fjname_js)
data = json.loads(data)

#获取番剧ID和中文译名
dataArray=data['list']
for i in range(len(dataArray)):
    id = int(dataArray[i]['id'])
    name_cn = dataArray[i]['name_cn']
    print(i+1," - ",name_cn)

#选择番剧
x = input("请确定番剧是否在目录中，并输入确定的序号：")
x = int(x)


#确定番剧ID
fjid = str(dataArray[x-1]['id'])

#获取番剧上映年份
url = "https://api.bgm.tv/v0/subjects/"+fjid
fjpage = get(url,headers=headers)

fjpage_js = fjpage.json()

fjpagedata = json.dumps(fjpage_js)
fjpagedata = json.loads(fjpagedata)

fjyear_str = fjpagedata['date']
fjyear = fjyear_str[0:4]

#更改文件夹名
name_cn_str = str(dataArray[x-1]['name_cn'])
folder2 = name_cn_str + " " + "(" + fjyear + ")"

    #校错，使用全角替换半角斜杠，避免抛出错误
if "/" in folder2:
    print("yes")
    folder2 = folder2.replace('/','／')

os.rename(folder,folder2)
folder=folder2



#获取番剧章节列表
url = "https://api.bgm.tv/v0/episodes?subject_id="+fjid+"&type=0&limit=100&offset=0"

fjeplist = get(url,headers=headers)

#格式化json
fjeplist_js = fjeplist.json()

fjepdata = json.dumps(fjeplist_js)
fjepdata = json.loads(fjepdata)

#读取番剧章节数据
dataArray=fjepdata['data']

#确定番剧章节数目
max=int(fjepdata['total'])

#更改工作目录
path = main_dir+"\\"+folder
os.chdir(path)

#调试用，查看工作目录是否正确
#print(os.getcwd())

#删除多余文件
print("即将删除nfo文件，图片，和metadata文件夹……")

# 函数代码来源https://www.programminghunter.com/article/84012371610/
def deleteimg(inrootpath):

    for root, dirs, files in os.walk(inrootpath):
        for file in files:
            file_path = os.path.join(root, file)
            #判断后缀是不是JPG结尾，是就删除
            if str(file_path.split('.')[-1]).upper()  == 'JPG':
                os.remove(file_path)
                print('删除{0}成功'.format(file_path))
            
            elif str(file_path.split('.')[-1]).upper()  == 'PNG':
                os.remove(file_path)
                print('删除{0}成功'.format(file_path))

            elif str(file_path.split('.')[-1]).upper()  == 'NFO':
                os.remove(file_path)
                print('删除{0}成功'.format(file_path))


deleteimg(path)

#判断metadata文件夹是否存在，若存在则删除。
if os.path.exists("metadata"):
    rmtree("metadata")


#打印预览版，确定是否有错误
print("↓↓↓↓↓请仔细检查，左为旧文件名，右为新文件名↓↓↓↓↓")
print("===")
i = 0
for item in os.scandir():
    if i < 9:
        name_cn = dataArray[i]['name_cn']
        hzm = os.path.splitext(item.name)[-1]
        out = "ep0"+str(i+1)+"-"+name_cn+hzm
        print(item.name,"==>",out)
    elif i >= max:
        break
    else:
        name_cn = dataArray[i]['name_cn']
        hzm = os.path.splitext(item.name)[-1]
        out = "ep"+str(i+1)+"-"+name_cn+hzm
        print(item.name,"==>",out)
    i=i+1
            
print("===")
print("↑↑↑↑↑请仔细检查，左为旧文件名，右为新文件名↑↑↑↑↑")

#以防万一，再删除一次
print("即将删除nfo文件，图片，和metadata文件夹……")

# 函数代码来源https://www.programminghunter.com/article/84012371610/
def deleteimg(inrootpath):

    for root, dirs, files in os.walk(inrootpath):
        for file in files:
            file_path = os.path.join(root, file)
            #判断后缀是不是JPG结尾，是就删除
            if str(file_path.split('.')[-1]).upper()  == 'JPG':
                os.remove(file_path)
                print('删除{0}成功'.format(file_path))
            
            elif str(file_path.split('.')[-1]).upper()  == 'PNG':
                os.remove(file_path)
                print('删除{0}成功'.format(file_path))

            elif str(file_path.split('.')[-1]).upper()  == 'NFO':
                os.remove(file_path)
                print('删除{0}成功'.format(file_path))


deleteimg(path)

#判断metadata文件夹是否存在，若存在则删除。
if os.path.exists("metadata"):
    rmtree("metadata")
    print("已删除metadata文件夹")

#开始重命名

i = 0
yn = input("请仔细检查预览表，确定是否开始重命名？(y/n)：")
if yn == "y":
    #设定番剧章节视频文件所在目录
    for item in os.scandir():
        
        if i < 9:
            name_cn=dataArray[i]['name_cn']
            hzm = os.path.splitext(item.name)[-1]
            out = "ep0"+str(i+1)+"-"+name_cn+hzm
            os.rename(item.name,out)
        elif i >= max:
            break
        else:
            name_cn = dataArray[i]['name_cn']
            hzm = os.path.splitext(item.name)[-1]
            out = "ep"+str(i+1)+"-"+name_cn+hzm
            os.rename(item.name,out)
        i=i+1
elif yn == "n":
    print("放弃更改，3秒后退出……")
    sleep(3)
    exit()
else:
    exit()
print("重命名成功！3秒后退出……")
sleep(3)
exit()