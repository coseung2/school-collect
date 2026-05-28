#!/usr/bin/env python3
"""
HWPX Mail Merge — 템플릿 + CSV 데이터 → 개인별 HWPX 대량 생성
===============================================================

사용법:
    python mailmerge.py <템플릿.hwpx> <데이터.csv> [출력폴더]

데이터 CSV 형식 (UTF-8 BOM):
    학년,반,번호,이름,성별,키,몸무게,표준체중,체질량지수,왼쪽시력,오른쪽시력,안경착용
    5,3,1,공서희,여,144.8,40.1,40.3,19.1,0.8,0.8,
    5,3,2,김민아,여,142.9,36.6,38.6,17.9,1.2,0.9,예

필수 금지 규칙:
    ❌ XML 문자열 replace/re.sub/f-string — 파일 깨짐
    ❌ linesegarray 미삭제 — 한글 글자 겹침
    ❌ 원본 덮어쓰기 — _agent_작업/에 사본
    ✅ lxml.etree로 파싱 → 수정 → 직렬화
    ✅ 수정한 모든 <hp:p>에서 <linesegarray> 반드시 삭제
    ✅ mimetype ZIP_STORED 첫 항목
"""

import csv
import os
import re
import shutil
import sys
import tempfile
import zipfile
from lxml import etree


# ─── 네임스페이스 ───────────────────────────────────────────────
NS = {
    'hp': 'http://www.hancom.co.kr/hwpml/2011/paragraph',
    'hc': 'http://www.hancom.co.kr/hwpml/2011/core',
}
HP = f'{{{NS["hp"]}}}'
HC = f'{{{NS["hc"]}}}'


def remove_linesegarray(p_elem):
    """★ 필수: 수정한 문단의 줄배치 캐시 삭제 (안 지우면 글자 겹침)"""
    for child in list(p_elem):
        if etree.QName(child.tag).localname == 'linesegarray':
            p_elem.remove(child)


def parse_csv(csv_path):
    """CSV(UTF-8 BOM) → list[dict]"""
    with open(csv_path, encoding='utf-8-sig') as f:
        reader = csv.DictReader(f)
        rows = list(reader)
    if not rows:
        raise ValueError("CSV가 비어 있습니다.")
    print(f"  ✓ 데이터 {len(rows)}건 로드")
    print(f"  ✓ 필드: {list(rows[0].keys())}")
    return rows


def analyze_template(template_path):
    """템플릿 HWPX에서 메일머지 필드 목록 추출"""
    with zipfile.ZipFile(template_path) as zf:
        with zf.open('Contents/section0.xml') as f:
            tree = etree.parse(f)
    
    root = tree.getroot()
    fields = set()
    
    # 방법 1: {{필드명}} 텍스트 검색
    for elem in root.iter(f'{HP}t'):
        if elem.text and re.match(r'\{\{.+?\}\}', elem.text.strip()):
            fields.add(elem.text.strip()[2:-2])
    
    # 방법 2: MAILMERGE fieldBegin의 Command/FieldValue 파라미터
    for fb in root.iter(f'{HP}fieldBegin'):
        if fb.get('type') == 'MAILMERGE':
            for param in fb.iter(f'{HP}stringParam'):
                if param.get('name') == 'Command' and param.text:
                    fields.add(param.text)
    
    return sorted(fields), tree, root


def do_mailmerge(template_path, csv_path, output_dir):
    """
    메인 함수: 템플릿 + CSV → 개인별 HWPX N개 생성
    
    Args:
        template_path: HWPX 템플릿 파일 경로
        csv_path: CSV 데이터 파일 경로 (UTF-8 BOM)
        output_dir: 출력 디렉토리
    """
    print("=" * 60)
    print("HWPX 메일머지 시작")
    print("=" * 60)
    
    # ─── 1. 템플릿 필드 분석 ───
    print("\n[1/4] 템플릿 분석 중...")
    fields, tree_tpl, root_tpl = analyze_template(template_path)
    print(f"  ✓ 발견된 필드: {fields}")
    
    # ─── 2. 데이터 로드 ───
    print("\n[2/4] 데이터 로드 중...")
    rows = parse_csv(csv_path)
    
    # 데이터 컬럼과 템플릿 필드 매칭 확인
    data_cols = set(rows[0].keys())
    missing = [f for f in fields if f not in data_cols]
    if missing:
        print(f"  ⚠  템플릿 필드 중 CSV에 없는 것: {missing}")
    else:
        print(f"  ✓ 모든 템플릿 필드가 CSV에 존재합니다")
    
    # ─── 3. 출력 폴더 준비 ───
    print("\n[3/4] 출력 폴더 준비 중...")
    os.makedirs(output_dir, exist_ok=True)
    print(f"  ✓ 출력 폴더: {output_dir}")
    
    # ─── 4. 개인별 문서 생성 ───
    print(f"\n[4/4] 문서 생성 중 ({len(rows)}건)...")
    
    for i, row in enumerate(rows, 1):
        name = row.get('이름', f'학생{i}')
        num = row.get('번호', str(i))
        grade = row.get('학년', '')
        ban = row.get('반', '')
        
        output_name = f"{grade}학년_{ban}반_{num}번_{name}.hwpx"
        output_path = os.path.join(output_dir, output_name)
        
        # 템플릿 복사해서 압축 해제
        work_dir = tempfile.mkdtemp(prefix='hwpx_')
        try:
            with zipfile.ZipFile(template_path) as zf:
                zf.extractall(work_dir)
            
            # section0.xml 수정
            xml_path = os.path.join(work_dir, 'Contents', 'section0.xml')
            tree = etree.parse(xml_path)
            root = tree.getroot()
            
            modify_count = 0
            
            # (A) {{필드}} 텍스트 교체
            for elem in root.iter(f'{HP}t'):
                if elem.text and (m := re.match(r'\{\{(.+?)\}\}', elem.text.strip())):
                    field = m.group(1)
                    if field in row and row[field].strip():
                        old_text = elem.text
                        elem.text = row[field].strip()
                        modify_count += 1
                        # 부모의 부모 = <hp:p> → linesegarray 삭제
                        p = elem.getparent().getparent()
                        remove_linesegarray(p)
            
            # (B) FieldValue/Command 파라미터 교체
            for param in root.iter(f'{HP}stringParam'):
                name_attr = param.get('name', '')
                if name_attr in ('FieldValue', 'Command') and param.text:
                    if param.text in row and row[param.text].strip():
                        param.text = row[param.text].strip()
            
            # 수정된 XML 저장
            tree.write(xml_path, xml_declaration=True, encoding='UTF-8')
            
            # HWPX 재패키징
            with zipfile.ZipFile(output_path, 'w', zipfile.ZIP_DEFLATED) as zout:
                # mimetype은 ZIP_STORED, 첫 번째 항목
                zout.writestr('mimetype', 'application/hwp+zip',
                              compress_type=zipfile.ZIP_STORED)
                for dirpath, _, filenames in os.walk(work_dir):
                    for fn in filenames:
                        fp = os.path.join(dirpath, fn)
                        arc_name = os.path.relpath(fp, work_dir)
                        if arc_name == 'mimetype':
                            continue
                        zout.write(fp, arc_name, compress_type=zipfile.ZIP_DEFLATED)
            
            print(f"  [{i}/{len(rows)}] {output_name}  (수정 {modify_count}건)")
            
        finally:
            # 작업 디렉토리 정리
            shutil.rmtree(work_dir, ignore_errors=True)
    
    # ─── 5. 검증 ───
    print("\n검증 중...")
    errors = []
    for fn in os.listdir(output_dir):
        if not fn.endswith('.hwpx'):
            continue
        fp = os.path.join(output_dir, fn)
        
        # ZIP 무결성
        try:
            with zipfile.ZipFile(fp) as zf:
                bad = zf.testzip()
                if bad:
                    errors.append(f"{fn}: ZIP 손상 ({bad})")
        except Exception as e:
            errors.append(f"{fn}: ZIP 열기 실패 ({e})")
            continue
        
        # 잔여 플레이스홀더 검사
        try:
            with zipfile.ZipFile(fp) as zf:
                xml = zf.read('Contents/section0.xml').decode()
                remaining = re.findall(r'\{\{.+?\}\}', xml)
                if remaining:
                    errors.append(f"{fn}: 남은 플레이스홀더 {remaining}")
        except Exception as e:
            errors.append(f"{fn}: XML 검증 실패 ({e})")
    
    print(f"  ✓ {len(os.listdir(output_dir))}개 파일 검증 완료")
    if errors:
        print(f"  ⚠  발견된 문제 {len(errors)}건:")
        for err in errors:
            print(f"    - {err}")
    else:
        print(f"  ✓ 모든 검증 통과!")
    
    print(f"\n{'=' * 60}")
    print(f"완료! 출력물: {output_dir}")
    print(f"{'=' * 60}")
    
    return errors


if __name__ == '__main__':
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(1)
    
    template = sys.argv[1]
    csv_data = sys.argv[2]
    output = sys.argv[3] if len(sys.argv) > 3 else '출력물'
    
    # 파일 존재 확인
    for p, label in [(template, '템플릿'), (csv_data, '데이터 CSV')]:
        if not os.path.exists(p):
            print(f"오류: {label} 파일 없음 → {p}")
            sys.exit(1)
    
    do_mailmerge(template, csv_data, output)
